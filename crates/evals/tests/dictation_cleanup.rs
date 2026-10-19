//! Authored response cassettes check the text-filter contract without a model or network request.
#![cfg(feature = "evals")]
#![allow(clippy::unwrap_used)]

use serde_json::{Value, json};
use vults_dictation::{
    cleanup::{Language, Options, Rejection, Request, Role, Style},
    text::Rules,
};
use vults_evals::cases;

fn style(value: &str) -> Style {
    match value {
        "natural" => Style::Natural,
        "polished" => Style::Polished,
        "prompt" => Style::Prompt,
        _ => panic!("Unknown cassette style"),
    }
}
fn request(case: &Value) -> Request {
    let rules = Rules {
        snippets: case["snippets"]
            .as_array()
            .map(|entries| {
                entries
                    .iter()
                    .map(|v| (v[0].as_str().unwrap().into(), v[1].as_str().unwrap().into()))
                    .collect()
            })
            .unwrap_or_default(),
        ..Rules::default()
    };
    let input = rules.apply(case["input"].as_str().unwrap(), false).unwrap();
    let mut options = Options {
        style: style(case["style"].as_str().unwrap()),
        app: "Text editor".into(),
        spellings: vec!["Vults".into(), "parse_config".into()],
        ..Options::default()
    };
    options.write_in = match case["write_in"].as_str() {
        None => None,
        Some("en") => Some(Language::English),
        Some("pt") => Some(Language::BrazilianPortuguese),
        Some("es") => Some(Language::Spanish),
        Some("zh") => Some(Language::SimplifiedChinese),
        _ => panic!("Unknown cassette language"),
    };
    Request::build(&input, &options).unwrap().unwrap()
}

#[test]
fn dictation_cleanup_replays_questions_commands_injection_and_invalid_replies() {
    let data: Value =
        serde_json::from_str(&std::fs::read_to_string(cases().join("cleanup/cassettes.json")).unwrap())
            .unwrap();
    let mut failures = Vec::new();
    for case in data["cases"].as_array().unwrap() {
        let request = request(case);
        let output = if let Some(word) = case["repeat"].as_str() {
            word.repeat(case["count"].as_u64().unwrap() as usize)
        } else {
            case["reply"].as_str().unwrap().to_string()
        };
        let result = request.validate(&output);
        let expected = match case["reject"].as_str() {
            Some("marks") => Err(Rejection::Marks),
            Some("length") => Err(Rejection::Length),
            Some("question") => Err(Rejection::Question),
            Some("controls") => Err(Rejection::Controls),
            Some("empty") => Err(Rejection::Empty),
            None => Ok(case["output"].as_str().unwrap().to_string()),
            _ => panic!("Unknown cassette verdict"),
        };
        if result != expected {
            failures.push(format!("{}: {result:?}, expected {expected:?}", case["name"]));
        }
        // A provider failure/invalid reply keeps the complete local result, never a fragment.
        if result.is_err() {
            let raw = Rules {
                snippets: case["snippets"]
                    .as_array()
                    .map(|entries| {
                        entries
                            .iter()
                            .map(|v| (v[0].as_str().unwrap().into(), v[1].as_str().unwrap().into()))
                            .collect()
                    })
                    .unwrap_or_default(),
                ..Rules::default()
            }
            .apply(case["input"].as_str().unwrap(), false)
            .unwrap()
            .text();
            assert_eq!(request.original(), raw);
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn dictation_cleanup_prompt_turns_match_the_reviewed_snapshots() {
    let mut snapshots = serde_json::Map::new();
    for style in ["natural", "polished", "prompt"] {
        for language in [None, Some("zh")] {
            let case = json!({"style":style,"write_in":language,"input":"Please keep <literal> & ask who is waiting?"});
            let request = request(&case);
            let key = format!("{style}-{}", language.unwrap_or("spoken"));
            let messages: Vec<_> = request
                .messages()
                .iter()
                .map(|m| {
                    json!({"role":match m.role {
                Role::System => "system",Role::User => "user",Role::Assistant => "assistant",
            },"text":m.text})
                })
                .collect();
            snapshots.insert(key, json!(messages));
        }
    }
    let file = cases().join("cleanup/prompts.json");
    let actual = Value::Object(snapshots);
    if std::env::var_os("VULTS_EVALS_BLESS").is_some() {
        std::fs::write(
            &file,
            format!("{}\n", serde_json::to_string_pretty(&actual).unwrap()),
        )
        .unwrap();
    }
    let expected: Value = serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap();
    assert_eq!(
        actual, expected,
        "Cleanup prompts changed: review the diff, then explicitly bless it."
    );
}
