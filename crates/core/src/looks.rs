//! Zeca's seasonal looks: a witch hat in October, a Santa hat until Christmas, a party hat at New
//! Year, bunny ears at Easter. The user picks Auto (the calendar), None, or one look for good.
//! The date comes in from the app as an [`crate::Input::Today`]: the core reads no clock. Ids name
//! the renderer's looks (`design/mascots/zeca/zeca.py`); they are saved in the settings, so they
//! never change.

use serde::{Deserialize, Serialize};

/// A day in the user's own calendar (their time zone, not UTC).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Date {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

impl Date {
    pub const fn new(year: i32, month: u8, day: u8) -> Self {
        Self { year, month, day }
    }

    /// Days since 1970-01-01 (proleptic Gregorian), to count days across a month's end.
    fn days(self) -> i64 {
        let (m, d) = (i64::from(self.month), i64::from(self.day));
        let y = i64::from(self.year) - i64::from(m <= 2);
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
        era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468
    }
}

/// What Zeca wears: the setting's choice. `Auto` follows the calendar; `None` is no look.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Outfit {
    #[default]
    Auto,
    None,
    WitchHat,
    SantaHat,
    PartyHat,
    BunnyEars,
    Sunglasses,
}

impl Outfit {
    /// The look the calendar gives `date`, if any. The windows do not overlap.
    pub fn seasonal(date: Date) -> Option<Outfit> {
        let Date { month, day, .. } = date;
        match (month, day) {
            (12, 31) | (1, 1..=2) => return Some(Outfit::PartyHat),
            (12, 1..=26) => return Some(Outfit::SantaHat),
            (10, _) | (11, 1) => return Some(Outfit::WitchHat),
            _ => {}
        }
        // Good Friday to Easter Monday.
        let from_easter = date.days() - easter(date.year).days();
        (-2..=1).contains(&from_easter).then_some(Outfit::BunnyEars)
    }

    /// What Zeca wears today: the calendar's look for `Auto` (none until the app says the date),
    /// nothing for `None`, the chosen look otherwise.
    pub fn worn(self, today: Option<Date>) -> Option<Outfit> {
        match self {
            Outfit::Auto => today.and_then(Outfit::seasonal),
            Outfit::None => None,
            look => Some(look),
        }
    }
}

/// Easter Sunday of `year` (Gregorian), by the Meeus/Jones/Butcher algorithm.
pub fn easter(year: i32) -> Date {
    let a = year % 19;
    let b = year / 100;
    let c = year % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let n = h + l - 7 * m + 114;
    // n / 31 is 3 or 4, n % 31 + 1 at most 31: both fit a u8.
    Date::new(year, (n / 31) as u8, (n % 31 + 1) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn on(year: i32, month: u8, day: u8) -> Option<Outfit> {
        Outfit::seasonal(Date::new(year, month, day))
    }

    #[test]
    fn halloween_runs_from_october_1_to_november_1() {
        assert_eq!(on(2026, 9, 30), None);
        assert_eq!(on(2026, 10, 1), Some(Outfit::WitchHat));
        assert_eq!(on(2026, 10, 31), Some(Outfit::WitchHat));
        assert_eq!(on(2026, 11, 1), Some(Outfit::WitchHat));
        assert_eq!(on(2026, 11, 2), None);
    }

    #[test]
    fn christmas_runs_from_december_1_to_26() {
        assert_eq!(on(2026, 11, 30), None);
        assert_eq!(on(2026, 12, 1), Some(Outfit::SantaHat));
        assert_eq!(on(2026, 12, 25), Some(Outfit::SantaHat));
        assert_eq!(on(2026, 12, 26), Some(Outfit::SantaHat));
        assert_eq!(on(2026, 12, 27), None);
    }

    #[test]
    fn new_year_runs_from_december_31_to_january_2() {
        assert_eq!(on(2026, 12, 30), None);
        assert_eq!(on(2026, 12, 31), Some(Outfit::PartyHat));
        assert_eq!(on(2027, 1, 1), Some(Outfit::PartyHat));
        assert_eq!(on(2027, 1, 2), Some(Outfit::PartyHat));
        assert_eq!(on(2027, 1, 3), None);
    }

    #[test]
    fn easter_by_the_computus() {
        let easters = [
            (2000, 4, 23),
            (2008, 3, 23), // early: before the spring equinox's month ends
            (2011, 4, 24),
            (2019, 4, 21),
            (2024, 3, 31),
            (2025, 4, 20),
            (2026, 4, 5),
            (2027, 3, 28),
            (2028, 4, 16),
            (2038, 4, 25), // the latest it can be
            (2285, 3, 22), // the earliest it can be
        ];
        for (y, m, d) in easters {
            assert_eq!(easter(y), Date::new(y, m, d), "{y}");
        }
    }

    #[test]
    fn bunny_ears_from_good_friday_to_easter_monday() {
        // Easter 2026 is April 5.
        assert_eq!(on(2026, 4, 2), None);
        for day in 3..=6 {
            assert_eq!(on(2026, 4, day), Some(Outfit::BunnyEars), "April {day}");
        }
        assert_eq!(on(2026, 4, 7), None);
        // Easter 2027 is March 28.
        assert_eq!(on(2027, 3, 25), None);
        assert_eq!(on(2027, 3, 26), Some(Outfit::BunnyEars));
        assert_eq!(on(2027, 3, 29), Some(Outfit::BunnyEars));
        assert_eq!(on(2027, 3, 30), None);
        // Across a month's end: Easter 2024 is March 31, its Monday April 1.
        assert_eq!(on(2024, 3, 29), Some(Outfit::BunnyEars));
        assert_eq!(on(2024, 4, 1), Some(Outfit::BunnyEars));
        assert_eq!(on(2024, 4, 2), None);
    }

    #[test]
    fn no_look_on_ordinary_days() {
        for (m, d) in [(1, 3), (2, 14), (6, 21), (7, 15), (9, 7)] {
            assert_eq!(on(2026, m, d), None, "{m}/{d}");
        }
    }

    #[test]
    fn days_count_across_years() {
        assert_eq!(Date::new(1970, 1, 1).days(), 0);
        assert_eq!(Date::new(2000, 3, 1).days() - Date::new(2000, 2, 28).days(), 2);
        assert_eq!(Date::new(2027, 1, 1).days() - Date::new(2026, 12, 31).days(), 1);
    }

    #[test]
    fn the_setting_picks_what_is_worn() {
        let halloween = Some(Date::new(2026, 10, 15));
        assert_eq!(Outfit::Auto.worn(halloween), Some(Outfit::WitchHat));
        assert_eq!(Outfit::Auto.worn(Some(Date::new(2026, 7, 15))), None);
        // No date yet: nothing, rather than a guess.
        assert_eq!(Outfit::Auto.worn(None), None);
        assert_eq!(Outfit::None.worn(halloween), None);
        assert_eq!(Outfit::Sunglasses.worn(halloween), Some(Outfit::Sunglasses));
        assert_eq!(Outfit::SantaHat.worn(None), Some(Outfit::SantaHat));
    }

    #[test]
    fn ids_are_stable() {
        let ids = [
            (Outfit::Auto, "auto"),
            (Outfit::None, "none"),
            (Outfit::WitchHat, "witch-hat"),
            (Outfit::SantaHat, "santa-hat"),
            (Outfit::PartyHat, "party-hat"),
            (Outfit::BunnyEars, "bunny-ears"),
            (Outfit::Sunglasses, "sunglasses"),
        ];
        for (outfit, id) in ids {
            assert_eq!(serde_json::to_value(outfit).ok(), Some(serde_json::json!(id)));
            assert_eq!(
                serde_json::from_value::<Outfit>(serde_json::json!(id)).ok(),
                Some(outfit)
            );
        }
        assert!(serde_json::from_value::<Outfit>(serde_json::json!("top-hat")).is_err());
    }
}
