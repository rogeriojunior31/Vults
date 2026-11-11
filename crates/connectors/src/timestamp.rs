//! RFC 3339 service timestamps, including fractional seconds and explicit UTC offsets.
pub(crate) fn parse(value: &str) -> Option<u64> {
    let b = value.as_bytes();
    if b.len() < 20 || b[4] != b'-' || b[7] != b'-' || b[10] != b'T' || b[13] != b':' || b[16] != b':' {
        return None;
    }
    if [(0, 4), (5, 7), (8, 10), (11, 13), (14, 16), (17, 19)]
        .iter()
        .any(|&(a, z)| !b[a..z].iter().all(u8::is_ascii_digit))
    {
        return None;
    }
    let number = |a, z| value.get(a..z)?.parse::<i64>().ok();
    let (mut y, m, d) = (number(0, 4)?, number(5, 7)?, number(8, 10)?);
    let (h, min, sec) = (number(11, 13)?, number(14, 16)?, number(17, 19)?);
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let days = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    };
    if !(1..=days).contains(&d) || h > 23 || min > 59 || sec > 59 {
        return None;
    }
    let mut tail = value.get(19..)?;
    if let Some(fraction) = tail.strip_prefix('.') {
        let digits = fraction.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 || digits > 9 {
            return None;
        }
        tail = fraction.get(digits..)?;
    }
    let offset = if tail == "Z" {
        0
    } else {
        let t = tail.as_bytes();
        if t.len() != 6 || !matches!(t[0], b'+' | b'-') || t[3] != b':' {
            return None;
        }
        if !t[1..3].iter().chain(t[4..6].iter()).all(u8::is_ascii_digit) {
            return None;
        }
        let hour = tail.get(1..3)?.parse::<i64>().ok()?;
        let minute = tail.get(4..6)?.parse::<i64>().ok()?;
        if hour > 23 || minute > 59 {
            return None;
        }
        (hour * 3600 + minute * 60) * if t[0] == b'-' { -1 } else { 1 }
    };
    if m <= 2 {
        y -= 1;
    }
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * ((m + 9) % 12) + 2) / 5 + d - 1;
    let days = era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468;
    u64::try_from(days * 86400 + h * 3600 + min * 60 + sec - offset).ok()
}
#[cfg(test)]
mod tests {
    #[test]
    fn fractions_offsets_and_calendar_dates_are_checked() {
        for value in [
            "1970-01-01T00:00:00Z",
            "1970-01-01T01:00:00.123+01:00",
            "1969-12-31T21:00:00-03:00",
        ] {
            assert_eq!(super::parse(value), Some(0), "{value}");
        }
        assert_eq!(super::parse("2000-03-01T00:00:00Z"), Some(951868800));
        for bad in [
            "2025-02-29T00:00:00Z",
            "2000-02-30T00:00:00Z",
            "2000-01-01T24:00:00Z",
            "2000-01-01T00:00:00.Z",
            "2000-01-01T00:00:00+25:00",
            "2000-01-01T00:00:00Zjunk",
        ] {
            assert_eq!(super::parse(bad), None, "{bad}");
        }
    }
}
