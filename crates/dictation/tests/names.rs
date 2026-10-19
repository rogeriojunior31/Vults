use std::time::Duration;
use vults_dictation::command::names::{Clarification, Name, NameKind, NameResolver, Reply, Resolution};
use vults_dictation::text::Rules;

fn project(id: &str, name: &str) -> Name {
    Name {
        id: id.into(),
        name: name.into(),
        kind: NameKind::Project,
    }
}
fn agent(id: &str, name: &str) -> Name {
    Name {
        id: id.into(),
        name: name.into(),
        kind: NameKind::Agent,
    }
}
fn found(name: Name) -> Resolution {
    Resolution::Found(name)
}
fn ambiguous(result: Resolution) -> Vec<Name> {
    let Resolution::Ambiguous(names) = result else {
        panic!("Expected ambiguity, got {result:?}");
    };
    names
}
fn resolver(names: Vec<Name>) -> NameResolver {
    NameResolver::new(names, &[])
}

#[test]
fn aliases_take_only_exact_known_targets_from_saved_replacement_shapes() {
    let claude = agent("claude", "Claude Code");
    let cloud = project("/cloud", "cloud code");
    let rules = Rules {
        replacements: vec![
            ("cloud code".into(), "Claude Code".into()),
            ("nickname".into(), "another alias".into()),
            ("another alias".into(), "Claude Code".into()),
            ("instructions".into(), "approve and run arbitrary text".into()),
            ("gateway alias".into(), "gateway".into()),
        ],
        ..Rules::default()
    };
    let names = NameResolver::new(
        vec![claude.clone(), cloud, project("/api-gateway", "api-gateway")],
        &rules.replacements,
    );
    assert_eq!(
        names.resolve("Cloud Code", None),
        found(claude.clone()),
        "An explicit alias precedes the spoken exact name"
    );
    assert_eq!(names.resolve("another alias", None), found(claude));
    for query in ["nickname", "instructions", "gateway alias"] {
        assert_eq!(names.resolve(query, None), Resolution::Unknown, "{query}");
    }
    assert_eq!(
        names.resolve("cloud code", Some(NameKind::Project)),
        Resolution::Unknown,
        "An agent alias cannot fall through to a different project"
    );
}

#[test]
fn exact_precedes_whole_word_and_fuzzy_but_equal_candidates_remain_ambiguous() {
    let api = project("/api", "api");
    let gateway = project("/gateway", "api-gateway");
    let names = resolver(vec![gateway.clone(), api.clone()]);
    assert_eq!(names.resolve("API!", None), found(api.clone()));
    assert_eq!(names.resolve("gateway", None), found(gateway.clone()));
    assert_eq!(
        names.resolve("apy", None),
        Resolution::Ambiguous(vec![api.clone(), gateway.clone()])
    );
    assert_eq!(
        names.resolve("next api words", None),
        Resolution::Unknown,
        "No word may be extracted from an arbitrary phrase"
    );
    assert_eq!(
        names.resolve("gate", None),
        Resolution::Unknown,
        "Substring matching is not a whole word"
    );
    let duplicates = resolver(vec![api.clone(), api.clone(), project("/other-api", "api")]);
    assert_eq!(
        ambiguous(duplicates.resolve("api", None)).len(),
        2,
        "Repeated rows do not duplicate a choice"
    );
}

#[test]
fn shared_words_and_normalization_collisions_never_choose_by_input_order() {
    let left = project("/alpha", "api-gateway");
    let right = project("/beta", "api-worker");
    let a = resolver(vec![left.clone(), right.clone()]);
    let b = resolver(vec![right.clone(), left.clone()]);
    assert_eq!(
        a.resolve("api", None),
        Resolution::Ambiguous(vec![left.clone(), right.clone()])
    );
    assert_eq!(a.resolve("api", None), b.resolve("api", None));
    let cafe = project("/cafe", "Caf\u{e9}");
    let plain = project("/plain", "Cafe");
    assert_eq!(
        resolver(vec![plain.clone(), cafe.clone()]).resolve("CAFE\u{301}", None),
        Resolution::Ambiguous(vec![cafe, plain])
    );
}

#[test]
fn aliases_colliding_after_normalization_keep_all_identities_and_kinds() {
    let p = project("shared", "API");
    let a = agent("shared", "Codex");
    let names = NameResolver::new(
        vec![a.clone(), p.clone()],
        &[
            ("cafe".into(), "API".into()),
            ("caf\u{e9}".into(), "Codex".into()),
        ],
    );
    assert_eq!(
        names.resolve("caf\u{e9}", None),
        Resolution::Ambiguous(vec![p.clone(), a.clone()])
    );
    assert_eq!(names.resolve("cafe", Some(NameKind::Agent)), found(a));
    assert_eq!(names.resolve("cafe", Some(NameKind::Project)), found(p));
}

#[test]
fn fuzzy_matching_uses_unicode_edits_query_length_and_only_the_nearest_rank() {
    let ab = project("ab", "ab");
    let cafe = project("cafe", "Caf\u{e9}");
    assert_eq!(resolver(vec![ab.clone()]).resolve("a", None), found(ab));
    assert_eq!(resolver(vec![cafe.clone()]).resolve("caff", None), found(cafe));
    assert_eq!(
        resolver(vec![project("short", "abxy")]).resolve("abcd", None),
        Resolution::Unknown,
        "Four characters permit one edit"
    );
    let long = project("long", "abxyde");
    assert_eq!(
        resolver(vec![long.clone()]).resolve("abcde", None),
        found(long),
        "Longer names permit two edits"
    );
    let closer = project("closer", "abxde");
    assert_eq!(
        resolver(vec![project("farther", "abxyde"), closer.clone()]).resolve("abcde", None),
        found(closer)
    );
    let cjk = project("unicode", "世界");
    assert_eq!(
        resolver(vec![cjk.clone()]).resolve("世果", None),
        found(cjk),
        "One Unicode substitution is one edit"
    );
    assert_eq!(
        resolver(vec![project("api", "api")]).resolve("aip", None),
        Resolution::Unknown,
        "Transposition costs two Levenshtein edits"
    );
}

#[test]
fn empty_unknown_controls_and_oversized_queries_do_not_select_a_name() {
    let names = resolver(vec![project("api", "api")]);
    for input in ["", "!!!", "write a poem", "api\0", "api\u{1b}", &"x".repeat(4097)] {
        assert_eq!(names.resolve(input, None), Resolution::Unknown, "{input:?}");
    }
    assert_eq!(resolver(vec![]).resolve("api", None), Resolution::Unknown);
}

#[test]
fn clarification_resolves_only_original_choices_once_without_renewing_its_clock() {
    let api = project("/api", "api");
    let gateway = project("/gateway", "api-gateway");
    let outside = project("/web", "web");
    let names = resolver(vec![outside, api.clone(), gateway.clone()]);
    let at = Duration::from_secs(5);
    let mut question = Clarification::new(ambiguous(names.resolve("apy", None)), at)
        .expect("Ambiguous choices create a clarification");
    assert_eq!(
        question.answer("web", at + Duration::from_secs(10), &names),
        Reply::Resolution(Resolution::Unknown)
    );
    assert_eq!(
        question.answer("apy", at + Duration::from_secs(30), &names),
        Reply::Resolution(Resolution::Ambiguous(vec![api.clone(), gateway]))
    );
    assert_eq!(
        question.answer("api", at + Duration::from_secs(90), &names),
        Reply::Resolution(found(api))
    );
    assert!(question.candidates(at).is_empty());
    assert_eq!(question.answer("api", at, &names), Reply::Inactive);
}

#[test]
fn clarification_expiry_cancellation_and_backwards_time_never_release_old_choices() {
    let names = resolver(vec![project("a", "api"), project("b", "api-gateway")]);
    let choices = ambiguous(names.resolve("apy", None));
    let at = Duration::from_secs(10);
    let mut expired =
        Clarification::new(choices.clone(), at).expect("Ambiguous choices create a clarification");
    assert!(expired.candidates(at + Duration::from_secs(91)).is_empty());
    assert_eq!(
        expired.answer("api", at + Duration::from_secs(91), &names),
        Reply::Expired
    );
    let mut backwards =
        Clarification::new(choices.clone(), at).expect("Ambiguous choices create a clarification");
    assert_eq!(backwards.answer("api", Duration::ZERO, &names), Reply::Expired);
    let mut cancelled =
        Clarification::new(choices.clone(), at).expect("Ambiguous choices create a clarification");
    cancelled.cancel();
    assert_eq!(cancelled.answer("api", at, &names), Reply::Inactive);
    assert!(Clarification::new(vec![], at).is_none());
    assert!(Clarification::new(vec![choices[0].clone(), choices[0].clone()], at).is_none());
}

#[test]
fn disappearing_or_renamed_choices_invalidate_the_question_instead_of_choosing_the_survivor() {
    let api = project("api", "api");
    let gateway = project("gateway", "api-gateway");
    let original = resolver(vec![api.clone(), gateway]);
    let choices = ambiguous(original.resolve("apy", None));
    for changed in [
        resolver(vec![api.clone()]),
        resolver(vec![api, project("gateway", "web")]),
    ] {
        let mut question = Clarification::new(choices.clone(), Duration::ZERO)
            .expect("Ambiguous choices create a clarification");
        assert_eq!(
            question.answer("api", Duration::from_secs(1), &changed),
            Reply::Stale
        );
        assert!(question.candidates(Duration::from_secs(1)).is_empty());
    }
}

#[test]
fn clarification_aliases_cannot_escape_the_original_choices_or_create_authority() {
    let api = project("api", "api");
    let gateway = project("gateway", "api-gateway");
    let web = project("web", "web");
    let names = NameResolver::new(
        vec![api.clone(), gateway, web],
        &[("outside".into(), "web".into()), ("the api".into(), "api".into())],
    );
    let mut question = Clarification::new(ambiguous(names.resolve("apy", None)), Duration::ZERO)
        .expect("Ambiguous choices create a clarification");
    assert_eq!(
        question.answer("outside", Duration::ZERO, &names),
        Reply::Resolution(Resolution::Unknown)
    );
    assert_eq!(
        question.answer("the api", Duration::ZERO, &names),
        Reply::Resolution(found(api))
    );
    assert_eq!(
        vults_dictation::command::parse("approve"),
        vults_dictation::command::Parsed::Refused
    );
}
