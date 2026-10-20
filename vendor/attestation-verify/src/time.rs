//! Hand-rolled RFC 3339 timestamp parsing, with no `chrono` or `time`
//! dependency.
//!
//! The trusted-root document's `validFor.start` / `validFor.end` fields are
//! the only place this crate needs calendar-time parsing. Everything else
//! (`integratedTime` and friends) is already a unix-seconds integer
//! encoded as a JSON string. A full calendar/timezone library is overkill
//! for "parse one RFC 3339 string, produce unix seconds," so this module
//! implements just that, by hand, with unit tests pinned against known-good
//! values.

use crate::error::ParseError;

/// Parses an RFC 3339 timestamp (`Z`/`z` or a numeric `+HH:MM`/`-HH:MM`
/// offset; fractional seconds accepted and rounded UP to the next whole
/// second) and returns the corresponding unix time in seconds.
///
/// Fractional seconds are rounded up (ceiling) rather than truncated. This is
/// exact for trust-window boundary comparisons: for integer-second evidence `t`
/// and a real boundary `b`, both `t >= b  ⟺  t >= ceil(b)` and
/// `t < b  ⟺  t < ceil(b)` hold. Truncation would widen the start-boundary
/// check by up to 999 ms, accepting evidence before the declared start.
///
/// `field` is used only to label the resulting [`ParseError::Rfc3339`] if
/// parsing fails.
pub(crate) fn parse_rfc3339(field: &'static str, s: &str) -> Result<i64, ParseError> {
    let bytes = s.as_bytes();
    let err = |reason: &str| ParseError::Rfc3339 {
        field,
        reason: reason.to_owned(),
    };

    // "YYYY-MM-DDTHH:MM:SSZ" is the shortest valid form.
    if bytes.len() < 20 {
        return Err(err("too short to be a valid RFC 3339 timestamp"));
    }

    let year = read_digits(bytes, 0, 4).ok_or_else(|| err("invalid year"))?;
    expect_byte(bytes, 4, b'-', &err)?;
    let month = read_digits(bytes, 5, 2).ok_or_else(|| err("invalid month"))?;
    expect_byte(bytes, 7, b'-', &err)?;
    let day = read_digits(bytes, 8, 2).ok_or_else(|| err("invalid day"))?;
    if bytes[10] != b'T' && bytes[10] != b't' {
        return Err(err("expected 'T' date/time separator"));
    }
    let hour = read_digits(bytes, 11, 2).ok_or_else(|| err("invalid hour"))?;
    expect_byte(bytes, 13, b':', &err)?;
    let minute = read_digits(bytes, 14, 2).ok_or_else(|| err("invalid minute"))?;
    expect_byte(bytes, 16, b':', &err)?;
    let second = read_digits(bytes, 17, 2).ok_or_else(|| err("invalid second"))?;

    if !(1..=12).contains(&month) {
        return Err(err("month out of range"));
    }
    if day < 1 || day > days_in_month(year, month) {
        return Err(err("day out of range for its month"));
    }
    if hour > 23 {
        return Err(err("hour out of range"));
    }
    if minute > 59 {
        return Err(err("minute out of range"));
    }
    if second > 59 {
        return Err(err("second out of range"));
    }

    let mut pos = 19;
    let mut has_nonzero_fraction = false;

    // Optional fractional seconds: '.' followed by one or more digits.
    // Rounded up (ceiling) to the next whole second if any digit is nonzero.
    if bytes[pos] == b'.' {
        pos += 1;
        let frac_start = pos;
        while pos < bytes.len() && bytes[pos].is_ascii_digit() {
            if bytes[pos] != b'0' {
                has_nonzero_fraction = true;
            }
            pos += 1;
        }
        if pos == frac_start {
            return Err(err("'.' must be followed by at least one digit"));
        }
    }

    if pos >= bytes.len() {
        return Err(err("missing timezone designator"));
    }

    let offset_seconds: i64 = match bytes[pos] {
        b'Z' | b'z' => {
            pos += 1;
            0
        }
        sign @ (b'+' | b'-') => {
            pos += 1;
            let offset_hour =
                read_digits(bytes, pos, 2).ok_or_else(|| err("invalid offset hours"))?;
            pos += 2;
            if pos >= bytes.len() || bytes[pos] != b':' {
                return Err(err("expected ':' in timezone offset"));
            }
            pos += 1;
            let offset_minute =
                read_digits(bytes, pos, 2).ok_or_else(|| err("invalid offset minutes"))?;
            pos += 2;
            if offset_hour > 23 || offset_minute > 59 {
                return Err(err("timezone offset out of range"));
            }
            let magnitude = i64::from(offset_hour) * 3600 + i64::from(offset_minute) * 60;
            if sign == b'-' { -magnitude } else { magnitude }
        }
        _ => return Err(err("expected 'Z' or a numeric timezone offset")),
    };

    if pos != bytes.len() {
        return Err(err("unexpected trailing characters"));
    }

    let days = days_from_civil(i64::from(year), month, day);
    let time_of_day_seconds = i64::from(hour) * 3600 + i64::from(minute) * 60 + i64::from(second);
    // `offset_seconds` is how far *ahead* of UTC the local reading is, so
    // UTC = local wall-clock reading (treated as if it were UTC) minus the
    // offset.
    let mut unix_time = days * 86_400 + time_of_day_seconds - offset_seconds;
    if has_nonzero_fraction {
        unix_time += 1;
    }
    Ok(unix_time)
}

fn expect_byte(
    bytes: &[u8],
    index: usize,
    expected: u8,
    err: &impl Fn(&str) -> ParseError,
) -> Result<(), ParseError> {
    if bytes.get(index) == Some(&expected) {
        Ok(())
    } else {
        Err(err("malformed RFC 3339 timestamp structure"))
    }
}

/// Reads exactly `n` ASCII digits starting at `start`, returning `None` if
/// out of bounds or any byte in range is not a digit.
fn read_digits(bytes: &[u8], start: usize, n: usize) -> Option<u32> {
    let end = start.checked_add(n)?;
    let slice = bytes.get(start..end)?;
    let mut value: u32 = 0;
    for &b in slice {
        if !b.is_ascii_digit() {
            return None;
        }
        value = value * 10 + u32::from(b - b'0');
    }
    Some(value)
}

fn is_leap_year(year: u32) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        // Unreachable given callers validate `month` is 1..=12 first; a
        // permissive fallback here would only ever make a bogus day
        // *more* likely to be rejected (0 days in "month"), never less.
        _ => 0,
    }
}

/// Days since the Unix epoch (1970-01-01) for a proleptic-Gregorian civil
/// date. `month` must be 1..=12 and `day` a valid day for that
/// month/year; callers validate both before calling this.
///
/// This is the well-known constant-time civil-calendar algorithm generally
/// credited to Howard Hinnant
/// (<http://howardhinnant.github.io/date_algorithms.html>), valid across
/// the entire proleptic Gregorian calendar. It is a mathematical
/// day-counting formula, re-derived here from its published description
/// rather than copied from any particular implementation.
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let year_of_era = y - era * 400; // [0, 399]
    let month_prime = (i64::from(month) + 9) % 12; // [0, 11]: Mar=0 .. Feb=11
    let day_of_year = (153 * month_prime + 2) / 5 + i64::from(day) - 1; // [0, 365]
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year; // [0, 146096]
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use super::parse_rfc3339;
    use crate::error::ParseError;

    // Reference values cross-checked against Python's
    // `datetime.fromisoformat(..).timestamp()`.

    #[test]
    fn parses_unix_epoch() -> Result<(), Box<dyn std::error::Error>> {
        if parse_rfc3339("f", "1970-01-01T00:00:00Z")? != 0 {
            return Err("epoch mismatch".into());
        }
        Ok(())
    }

    #[test]
    fn parses_pre_epoch_instant() -> Result<(), Box<dyn std::error::Error>> {
        if parse_rfc3339("f", "1969-12-31T23:59:59Z")? != -1 {
            return Err("pre-epoch mismatch".into());
        }
        Ok(())
    }

    #[test]
    fn parses_real_fixture_timestamp() -> Result<(), Box<dyn std::error::Error>> {
        if parse_rfc3339("f", "2021-03-07T03:20:29Z")? != 1_615_087_229 {
            return Err("fixture timestamp mismatch".into());
        }
        Ok(())
    }

    #[test]
    fn rounds_fractional_seconds_up() -> Result<(), Box<dyn std::error::Error>> {
        if parse_rfc3339("f", "2022-12-31T23:59:59.999Z")? != 1_672_531_200 {
            return Err("fractional-second rounding mismatch".into());
        }
        Ok(())
    }

    #[test]
    fn all_zero_fraction_unchanged() -> Result<(), Box<dyn std::error::Error>> {
        if parse_rfc3339("f", "2022-12-31T23:59:59.000Z")? != 1_672_531_199 {
            return Err("all-zero fraction should not round up".into());
        }
        Ok(())
    }

    #[test]
    fn single_digit_fraction_rounds_up() -> Result<(), Box<dyn std::error::Error>> {
        if parse_rfc3339("f", "2022-12-31T23:59:59.001Z")? != 1_672_531_200 {
            return Err("single-digit fraction mismatch".into());
        }
        Ok(())
    }

    #[test]
    fn many_digit_fraction_rounds_up() -> Result<(), Box<dyn std::error::Error>> {
        if parse_rfc3339("f", "2022-12-31T23:59:59.0000001Z")? != 1_672_531_200 {
            return Err("many-digit fraction mismatch".into());
        }
        Ok(())
    }

    #[test]
    fn parses_far_future_timestamp() -> Result<(), Box<dyn std::error::Error>> {
        if parse_rfc3339("f", "2025-09-23T00:00:00Z")? != 1_758_585_600 {
            return Err("far-future timestamp mismatch".into());
        }
        Ok(())
    }

    #[test]
    fn accepts_leap_day_on_leap_year() -> Result<(), Box<dyn std::error::Error>> {
        if parse_rfc3339("f", "2000-02-29T00:00:00Z")? != 951_782_400 {
            return Err("leap day mismatch".into());
        }
        Ok(())
    }

    #[test]
    fn accepts_century_leap_year_2400() -> Result<(), Box<dyn std::error::Error>> {
        if parse_rfc3339("f", "2400-01-01T00:00:00Z")? != 13_569_465_600 {
            return Err("2400 leap-century mismatch".into());
        }
        Ok(())
    }

    #[test]
    fn rejects_leap_day_on_non_leap_year() -> Result<(), Box<dyn std::error::Error>> {
        expect_rfc3339_error("2021-02-29T00:00:00Z")
    }

    #[test]
    fn rejects_leap_day_on_century_non_leap_year() -> Result<(), Box<dyn std::error::Error>> {
        // 1900 is divisible by 100 but not 400: not a leap year.
        expect_rfc3339_error("1900-02-29T00:00:00Z")
    }

    #[test]
    fn accepts_positive_numeric_offset() -> Result<(), Box<dyn std::error::Error>> {
        if parse_rfc3339("f", "2021-01-01T00:00:00+09:00")? != 1_609_426_800 {
            return Err("positive offset mismatch".into());
        }
        Ok(())
    }

    #[test]
    fn accepts_negative_numeric_offset() -> Result<(), Box<dyn std::error::Error>> {
        if parse_rfc3339("f", "2021-01-01T00:00:00-05:00")? != 1_609_477_200 {
            return Err("negative offset mismatch".into());
        }
        Ok(())
    }

    #[test]
    fn rejects_april_31() -> Result<(), Box<dyn std::error::Error>> {
        expect_rfc3339_error("2021-04-31T00:00:00Z")
    }

    #[test]
    fn rejects_month_zero() -> Result<(), Box<dyn std::error::Error>> {
        expect_rfc3339_error("2021-00-01T00:00:00Z")
    }

    #[test]
    fn rejects_month_thirteen() -> Result<(), Box<dyn std::error::Error>> {
        expect_rfc3339_error("2021-13-01T00:00:00Z")
    }

    #[test]
    fn rejects_hour_24() -> Result<(), Box<dyn std::error::Error>> {
        expect_rfc3339_error("2021-01-01T24:00:00Z")
    }

    #[test]
    fn rejects_missing_timezone() -> Result<(), Box<dyn std::error::Error>> {
        expect_rfc3339_error("2021-01-01T00:00:00")
    }

    #[test]
    fn rejects_bad_offset_minutes() -> Result<(), Box<dyn std::error::Error>> {
        expect_rfc3339_error("2021-01-01T00:00:00+09:60")
    }

    #[test]
    fn rejects_trailing_garbage() -> Result<(), Box<dyn std::error::Error>> {
        expect_rfc3339_error("2021-01-01T00:00:00Zgarbage")
    }

    #[test]
    fn rejects_empty_string() -> Result<(), Box<dyn std::error::Error>> {
        expect_rfc3339_error("")
    }

    #[test]
    fn rejects_non_ascii_digits() -> Result<(), Box<dyn std::error::Error>> {
        // Multi-byte UTF-8 in place of ASCII digits must be rejected, not
        // panic on a non-char-boundary slice.
        expect_rfc3339_error("२०२१-01-01T00:00:00Z")
    }

    fn expect_rfc3339_error(input: &str) -> Result<(), Box<dyn std::error::Error>> {
        match parse_rfc3339("f", input) {
            Err(ParseError::Rfc3339 { .. }) => Ok(()),
            other => Err(format!("expected Rfc3339 error for {input:?}, got {other:?}").into()),
        }
    }
}
