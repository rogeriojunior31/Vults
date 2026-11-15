//! The update notice's rules (FF4, ADR 0019), pure so they can be tested without a clock or the
//! network: which release tags count, which one is newer, when a check is due (rule 4: at most
//! daily), when the consent question comes back and how *Later* hides a version.

use serde::Deserialize;

/// The desktop app's release tags (`release.yml` runs on `v*`). Any other platform's releases use
/// another prefix and never match this one.
pub const TAG_PREFIX: &str = "v";
pub const DAY_MS: i64 = 24 * 3600 * 1000;
/// The first look after launch waits this long, so it never lands on top of the greeting.
pub const FIRST_LOOK_MS: i64 = 30_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    /// `MAJOR.MINOR.PATCH` and nothing else: a pre-release (`-rc.1`) or build (`+abc`) suffix is
    /// refused, so it is never offered as an update.
    pub fn parse(text: &str) -> Option<Self> {
        let mut parts = text.split('.');
        let mut next = || {
            let part = parts.next()?;
            // Leading zeros and signs are not version numbers; `u32::from_str` would accept "+1".
            let plain = !part.is_empty()
                && part.bytes().all(|b| b.is_ascii_digit())
                && (part == "0" || !part.starts_with('0'));
            if plain { part.parse().ok() } else { None }
        };
        let version = Self {
            major: next()?,
            minor: next()?,
            patch: next()?,
        };
        parts.next().is_none().then_some(version)
    }

    /// The running build's version.
    pub fn current() -> Option<Self> {
        Self::parse(env!("CARGO_PKG_VERSION"))
    }

    /// A tag of this platform's releases; rolling tags (`latest`, `nightly`) do not parse.
    pub fn from_tag(tag: &str) -> Option<Self> {
        Self::parse(tag.strip_prefix(TAG_PREFIX)?)
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// The fields read from one entry of GitHub's releases list; the rest is ignored.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct Release {
    pub tag_name: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub html_url: String,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub prerelease: bool,
}

/// The newest published release above `current`, if any. Drafts and pre-releases never count,
/// whatever their tag says.
pub fn newest(releases: &[Release], current: Version) -> Option<(Version, &Release)> {
    releases
        .iter()
        .filter(|r| !r.draft && !r.prerelease)
        .filter_map(|r| Some((Version::from_tag(&r.tag_name)?, r)))
        .filter(|(v, _)| *v > current)
        .max_by_key(|(v, _)| *v)
}

/// Is an automatic check due? At most one a day (rule 4). A last check stamped more than a day in
/// the future means the clock was wrong then; waiting for it would stop checks for good.
pub fn should_check(now_ms: i64, last_check_ms: Option<i64>) -> bool {
    match last_check_ms {
        None => true,
        Some(last) => now_ms.saturating_sub(last) >= DAY_MS || last.saturating_sub(now_ms) > DAY_MS,
    }
}

/// Whether the user let Vults look at its releases. Nothing is requested before `Yes`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase", tag = "state")]
pub enum Consent {
    #[default]
    Unasked,
    /// Asked and neither answered nor dismissed with an answer.
    Asked {
        at_ms: i64,
    },
    Yes,
    No,
}

impl Consent {
    pub fn allows_checks(self) -> bool {
        self == Self::Yes
    }

    /// Should the island ask now? Once; a question left unanswered comes back after a day.
    pub fn should_ask(self, now_ms: i64) -> bool {
        match self {
            Self::Unasked => true,
            Self::Asked { at_ms } => {
                now_ms.saturating_sub(at_ms) >= DAY_MS || at_ms.saturating_sub(now_ms) > DAY_MS
            }
            Self::Yes | Self::No => false,
        }
    }
}

/// *Later* on a notice: that version stays hidden for a day.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snooze {
    pub version: Version,
    pub until_ms: i64,
}

impl Snooze {
    pub fn later(version: Version, now_ms: i64) -> Self {
        Self {
            version,
            until_ms: now_ms.saturating_add(DAY_MS),
        }
    }
}

/// Should the notice for `found` show? A newer version than the snoozed one shows at once.
pub fn should_show(found: Version, snooze: Option<Snooze>, now_ms: i64) -> bool {
    snooze.is_none_or(|s| {
        found > s.version || now_ms >= s.until_ms || s.until_ms.saturating_sub(now_ms) > DAY_MS
    })
}

// Serialized as "0.1.9" so the saved settings stay readable.
impl serde::Serialize for Version {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}
impl<'de> Deserialize<'de> for Version {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let text = String::deserialize(d)?;
        Self::parse(&text).ok_or_else(|| serde::de::Error::custom("invalid version"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> Version {
        Version::parse(text).unwrap()
    }
    fn release(tag: &str) -> Release {
        Release {
            tag_name: tag.into(),
            html_url: format!("https://example.invalid/{tag}"),
            ..Release::default()
        }
    }

    #[test]
    fn only_plain_versions_parse() {
        assert_eq!(
            v("0.1.9"),
            Version {
                major: 0,
                minor: 1,
                patch: 9
            }
        );
        assert_eq!(v("10.20.30").to_string(), "10.20.30");
        for bad in [
            "",
            "1",
            "1.2",
            "1.2.3.4",
            "1.2.3-rc.1",
            "1.2.3+abc",
            "01.2.3",
            "1.+2.3",
            "1.2.x",
            " 1.2.3",
            "1..3",
        ] {
            assert_eq!(Version::parse(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn the_running_version_parses() {
        assert!(Version::current().is_some());
    }

    #[test]
    fn tags_need_this_platforms_prefix() {
        assert_eq!(Version::from_tag("v0.1.9"), Some(v("0.1.9")));
        for bad in [
            "0.1.9",
            "android-v0.2.0",
            "V0.1.9",
            "latest",
            "nightly",
            "v0.2.0-rc.1",
            "vv0.1.9",
        ] {
            assert_eq!(Version::from_tag(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn versions_order_by_number_not_text() {
        assert!(v("0.1.10") > v("0.1.9"));
        assert!(v("0.2.0") > v("0.1.99"));
        assert!(v("1.0.0") > v("0.99.99"));
    }

    #[test]
    fn newest_skips_drafts_prereleases_and_foreign_tags() {
        let mut draft = release("v0.4.0");
        draft.draft = true;
        let mut pre = release("v0.3.0");
        pre.prerelease = true;
        let list = [
            release("v0.1.7"),
            release("v0.2.1"),
            draft,
            pre,
            release("v0.3.0-rc.1"),
            release("android-v9.0.0"),
            release("nightly"),
            release("v0.2.0"),
        ];
        let (found, r) = newest(&list, v("0.1.8")).unwrap();
        assert_eq!(found, v("0.2.1"));
        assert_eq!(r.tag_name, "v0.2.1");
        assert_eq!(
            newest(&list, v("0.2.1")),
            None,
            "the same version is not an update"
        );
        assert_eq!(
            newest(&list, v("1.0.0")),
            None,
            "an older release is not an update"
        );
        assert_eq!(newest(&[], v("0.1.8")), None);
    }

    #[test]
    fn releases_list_parses_with_unknown_fields() {
        let raw = r#"[{"tag_name":"v0.1.9","name":"Vults v0.1.9","body":"Notes","html_url":"https://x","draft":false,"prerelease":false,"assets":[],"id":1},{"tag_name":"v0.2.0"}]"#;
        let list: Vec<Release> = serde_json::from_str(raw).unwrap();
        assert_eq!(list[0].body.as_deref(), Some("Notes"));
        assert_eq!(newest(&list, v("0.1.8")).unwrap().0, v("0.2.0"));
    }

    #[test]
    fn checks_are_at_most_daily() {
        let t = 1_000 * DAY_MS;
        assert!(should_check(t, None));
        assert!(!should_check(t, Some(t)));
        assert!(!should_check(t + DAY_MS - 1, Some(t)));
        assert!(should_check(t + DAY_MS, Some(t)));
        // A clock set back a little waits; a stamp far in the future does not stop checks forever.
        assert!(!should_check(t - 1_000, Some(t)));
        assert!(should_check(t - DAY_MS - 1, Some(t)));
    }

    #[test]
    fn the_question_is_asked_once_and_again_after_a_day_unanswered() {
        let t = 1_000 * DAY_MS;
        assert!(Consent::Unasked.should_ask(t));
        assert!(!Consent::Asked { at_ms: t }.should_ask(t + DAY_MS - 1));
        assert!(Consent::Asked { at_ms: t }.should_ask(t + DAY_MS));
        assert!(!Consent::Yes.should_ask(t));
        assert!(!Consent::No.should_ask(t + 100 * DAY_MS));
        assert!(Consent::Yes.allows_checks());
        for c in [Consent::Unasked, Consent::Asked { at_ms: t }, Consent::No] {
            assert!(!c.allows_checks(), "{c:?}");
        }
    }

    #[test]
    fn later_hides_that_version_for_a_day_and_a_newer_one_shows_at_once() {
        let t = 1_000 * DAY_MS;
        let s = Snooze::later(v("0.1.9"), t);
        assert!(should_show(v("0.1.9"), None, t));
        assert!(!should_show(v("0.1.9"), Some(s), t));
        assert!(!should_show(v("0.1.9"), Some(s), t + DAY_MS - 1));
        assert!(should_show(v("0.1.9"), Some(s), t + DAY_MS));
        assert!(should_show(v("0.1.10"), Some(s), t + 1));
        assert!(should_show(
            v("0.1.9"),
            Some(Snooze {
                version: v("0.1.9"),
                until_ms: t + 5 * DAY_MS
            }),
            t
        ));
    }

    #[test]
    fn state_round_trips_through_settings_json() {
        let s = Snooze::later(v("0.1.9"), 5);
        let raw = serde_json::to_string(&s).unwrap();
        assert_eq!(raw, r#"{"version":"0.1.9","untilMs":86400005}"#);
        assert_eq!(serde_json::from_str::<Snooze>(&raw).unwrap(), s);
        for c in [
            Consent::Unasked,
            Consent::Asked { at_ms: 7 },
            Consent::Yes,
            Consent::No,
        ] {
            let raw = serde_json::to_string(&c).unwrap();
            assert_eq!(serde_json::from_str::<Consent>(&raw).unwrap(), c, "{raw}");
        }
        assert!(serde_json::from_str::<Snooze>(r#"{"version":"0.1.9-rc.1","untilMs":1}"#).is_err());
    }
}
