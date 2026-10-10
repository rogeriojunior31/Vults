//! Days and weeks in the user's own calendar, from Unix seconds and the UTC offset the app reads
//! from the desktop: the core reads no clock and no time zone. Weeks are ISO's, Monday to Sunday.

use crate::looks::Date;

const DAY: i64 = 24 * 60 * 60;

impl Date {
    /// The day `days` after 1970-01-01 (proleptic Gregorian); the inverse of [`Date::days`].
    pub fn from_days(days: i64) -> Self {
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = doy - (153 * mp + 2) / 5 + 1;
        let month = if mp < 10 { mp + 3 } else { mp - 9 };
        let year = yoe + era * 400 + i64::from(month <= 2);
        Self::new(
            i32::try_from(year).unwrap_or(i32::MAX),
            u8::try_from(month).unwrap_or(1),
            u8::try_from(day).unwrap_or(1),
        )
    }

    /// The local day of an instant: Unix seconds, and the UTC offset in seconds then.
    pub fn of(unix: i64, offset: i32) -> Self {
        Self::from_days((unix + i64::from(offset)).div_euclid(DAY))
    }

    /// 0 for Monday … 6 for Sunday.
    pub fn weekday(self) -> u8 {
        // 1970-01-01 was a Thursday.
        u8::try_from((self.days() + 3).rem_euclid(7)).unwrap_or(0)
    }

    /// The Monday of its ISO week.
    pub fn monday(self) -> Self {
        Self::from_days(self.days() - i64::from(self.weekday()))
    }

    pub fn plus(self, days: i64) -> Self {
        Self::from_days(self.days() + days)
    }

    /// `2026-10-09`.
    pub fn iso(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }

    /// From `2026-10-09`; none for anything else, or a day that does not exist.
    pub fn parse(text: &str) -> Option<Self> {
        let mut parts = text.splitn(3, '-');
        let year = parts.next()?.parse().ok()?;
        let month = parts.next()?.parse().ok()?;
        let day = parts.next()?.parse().ok()?;
        let date = Self::new(year, month, day);
        (Self::from_days(date.days()) == date && text.len() == 10).then_some(date)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_round_trip_across_centuries_and_leap_days() {
        for days in (-800_000..800_000).step_by(997) {
            assert_eq!(Date::from_days(days).days(), days);
        }
        assert_eq!(Date::from_days(0), Date::new(1970, 1, 1));
        assert_eq!(Date::new(2024, 2, 29).plus(1), Date::new(2024, 3, 1));
        assert_eq!(Date::new(2026, 12, 31).plus(1), Date::new(2027, 1, 1));
    }

    #[test]
    fn a_local_day_follows_the_offset() {
        // 2026-10-09 23:30 UTC.
        let t = 1_791_588_600;
        assert_eq!(Date::of(t, 0), Date::new(2026, 10, 9));
        assert_eq!(
            Date::of(t, 3600),
            Date::new(2026, 10, 10),
            "UTC+1 is past midnight"
        );
        assert_eq!(Date::of(t, -3 * 3600), Date::new(2026, 10, 9), "UTC-3");
    }

    #[test]
    fn iso_weeks_start_on_monday() {
        // 2026-10-09 is a Friday.
        let friday = Date::new(2026, 10, 9);
        assert_eq!(friday.weekday(), 4);
        assert_eq!(friday.monday(), Date::new(2026, 10, 5));
        assert_eq!(Date::new(2026, 10, 5).monday(), Date::new(2026, 10, 5));
        assert_eq!(Date::new(2026, 10, 11).weekday(), 6);
        assert_eq!(Date::new(2026, 10, 11).monday(), Date::new(2026, 10, 5));
        // Across a year's end.
        assert_eq!(Date::new(2027, 1, 1).monday(), Date::new(2026, 12, 28));
    }

    #[test]
    fn iso_text_round_trips_and_nonsense_is_refused() {
        let d = Date::new(2026, 3, 7);
        assert_eq!(d.iso(), "2026-03-07");
        assert_eq!(Date::parse("2026-03-07"), Some(d));
        for bad in ["2026-02-30", "2026-13-01", "26-03-07", "2026-3-7", "", "x"] {
            assert_eq!(Date::parse(bad), None, "{bad}");
        }
    }
}
