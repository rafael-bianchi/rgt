use chrono::Duration;

const MAX_TOKEN_BYTES: usize = 128;
const MAX_EXPONENT: i32 = 128;
const DISPLAY_DIGITS: u32 = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DurationUnit {
    Seconds,
    Minutes,
    Hours,
    Days,
    Weeks,
}

impl DurationUnit {
    pub fn parse(name: &str) -> Result<Self, String> {
        match name {
            "seconds" => Ok(Self::Seconds),
            "minutes" => Ok(Self::Minutes),
            "hours" => Ok(Self::Hours),
            "days" => Ok(Self::Days),
            "weeks" => Ok(Self::Weeks),
            _ => Err(format!(
                "unsupported duration unit '{name}'; supported units: seconds, minutes, hours, days, weeks"
            )),
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Seconds => "seconds",
            Self::Minutes => "minutes",
            Self::Hours => "hours",
            Self::Days => "days",
            Self::Weeks => "weeks",
        }
    }

    pub const fn seconds_per_unit(self) -> i64 {
        match self {
            Self::Seconds => 1,
            Self::Minutes => 60,
            Self::Hours => 3_600,
            Self::Days => 86_400,
            Self::Weeks => 604_800,
        }
    }

    pub fn display(self, seconds: i64) -> DurationDisplay {
        let scale = self.seconds_per_unit() as u128;
        let absolute = seconds.unsigned_abs() as u128;
        let mut whole = absolute / scale;
        let remainder = absolute % scale;
        let decimal_scale = 10_u128.pow(DISPLAY_DIGITS);
        let scaled_remainder = remainder * decimal_scale;
        let finite = scaled_remainder.rem_euclid(scale) == 0;
        let fraction = if finite {
            scaled_remainder / scale
        } else {
            (scaled_remainder + scale / 2) / scale
        };
        let fraction = if fraction == decimal_scale {
            whole += 1;
            0
        } else {
            fraction
        };

        let mut decimal = if fraction == 0 {
            whole.to_string()
        } else {
            let fractional = format!("{fraction:0width$}", width = DISPLAY_DIGITS as usize);
            format!("{whole}.{}", fractional.trim_end_matches('0'))
        };
        if seconds < 0 && decimal != "0" {
            decimal.insert(0, '-');
        }

        let label = if whole == 1 && fraction == 0 {
            self.name().trim_end_matches('s')
        } else {
            self.name()
        };
        let text = if finite {
            format!("{decimal} {label}")
        } else {
            format!("approximately {decimal} {label}")
        };

        DurationDisplay {
            unit: self,
            decimal,
            approximate: !finite,
            text,
            duration_seconds: seconds.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurationDisplay {
    pub unit: DurationUnit,
    pub decimal: String,
    pub approximate: bool,
    pub text: String,
    pub duration_seconds: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurationClaim {
    lexeme: String,
    unit: DurationUnit,
    exact_seconds: i64,
}

impl DurationClaim {
    pub fn parse(lexeme: &str, unit: DurationUnit) -> Result<Self, String> {
        let exact_seconds = parse_exact_seconds(lexeme, unit)?;
        Ok(Self {
            lexeme: lexeme.to_string(),
            unit,
            exact_seconds,
        })
    }

    pub fn lexeme(&self) -> &str {
        &self.lexeme
    }

    pub const fn unit(&self) -> DurationUnit {
        self.unit
    }

    pub const fn exact_seconds(&self) -> i64 {
        self.exact_seconds
    }

    pub fn verify(&self, expected_seconds: i64) -> Result<(), String> {
        if self.exact_seconds() == expected_seconds {
            Ok(())
        } else {
            Err(format!(
                "expected {expected_seconds} seconds, supplied {} seconds (claim unit {}, token '{}')",
                self.exact_seconds(),
                self.unit().name(),
                self.lexeme()
            ))
        }
    }
}

fn parse_exact_seconds(lexeme: &str, unit: DurationUnit) -> Result<i64, String> {
    if lexeme.is_empty() || lexeme.len() > MAX_TOKEN_BYTES {
        return Err(format!(
            "duration claim must contain 1 to {MAX_TOKEN_BYTES} bytes"
        ));
    }

    let (negative, unsigned) = match lexeme.as_bytes()[0] {
        b'-' => (true, &lexeme[1..]),
        b'+' => (false, &lexeme[1..]),
        _ => (false, lexeme),
    };
    if unsigned.is_empty() {
        return Err("duration claim has a sign but no digits".into());
    }

    let exponent_at = unsigned.find(['e', 'E']);
    let (mantissa, exponent_text) = match exponent_at {
        Some(index) => {
            if unsigned[index + 1..].contains(['e', 'E']) {
                return Err("duration claim has more than one exponent marker".into());
            }
            (&unsigned[..index], Some(&unsigned[index + 1..]))
        }
        None => (unsigned, None),
    };
    let exponent = match exponent_text {
        Some(text) => parse_bounded_exponent(text)?,
        None => 0,
    };

    let mut digits = Vec::with_capacity(mantissa.len());
    let mut fractional_digits = 0_i32;
    let mut after_decimal = false;
    let mut saw_digit = false;
    for byte in mantissa.bytes() {
        match byte {
            b'0'..=b'9' => {
                saw_digit = true;
                digits.push(byte);
                if after_decimal {
                    fractional_digits += 1;
                }
            }
            b'.' if !after_decimal => after_decimal = true,
            _ => return Err("duration claim is not a valid decimal number".into()),
        }
    }
    if !saw_digit {
        return Err("duration claim must contain at least one digit".into());
    }

    let Some(first_nonzero) = digits.iter().position(|digit| *digit != b'0') else {
        return Ok(0);
    };
    let mut significant_digits = digits[first_nonzero..].to_vec();
    let trailing_zeros = significant_digits
        .iter()
        .rev()
        .take_while(|digit| **digit == b'0')
        .count() as i32;
    significant_digits.truncate(significant_digits.len() - trailing_zeros as usize);

    let scale = unit.seconds_per_unit() as u32;
    let mut scaled_digits = multiply_decimal_digits(&significant_digits, scale);
    let decimal_power = exponent
        .checked_sub(fractional_digits)
        .and_then(|power| power.checked_add(trailing_zeros))
        .ok_or_else(|| "duration claim exponent is out of range".to_string())?;
    if decimal_power >= 0 {
        let appended_zeros = decimal_power as usize;
        if scaled_digits.len().saturating_add(appended_zeros) > 19 {
            return Err("duration claim is outside the supported seconds range".into());
        }
        scaled_digits.resize(scaled_digits.len() + appended_zeros, b'0');
    } else {
        let removed_digits = decimal_power.unsigned_abs() as usize;
        if removed_digits >= scaled_digits.len() {
            return Err("duration claim does not represent a whole number of seconds".into());
        }
        let integer_digits = scaled_digits.len() - removed_digits;
        if scaled_digits[integer_digits..]
            .iter()
            .any(|digit| *digit != b'0')
        {
            return Err("duration claim does not represent a whole number of seconds".into());
        }
        scaled_digits.truncate(integer_digits);
    }

    if scaled_digits.len() > 19 {
        return Err("duration claim is outside the supported seconds range".into());
    }
    let magnitude = std::str::from_utf8(&scaled_digits)
        .map_err(|_| "duration claim contains invalid decimal digits".to_string())?
        .parse::<i128>()
        .map_err(|_| "duration claim is outside the supported seconds range".to_string())?;
    let signed_seconds = if negative { -magnitude } else { magnitude };
    let seconds = i64::try_from(signed_seconds)
        .map_err(|_| "duration claim is outside the supported seconds range".to_string())?;
    Duration::try_seconds(seconds)
        .ok_or_else(|| "duration claim is outside the supported Duration range".to_string())?;
    Ok(seconds)
}

fn multiply_decimal_digits(digits: &[u8], multiplier: u32) -> Vec<u8> {
    let mut product = Vec::with_capacity(digits.len() + 6);
    let mut carry = 0_u32;
    for digit in digits.iter().rev() {
        let value = (u32::from(*digit - b'0') * multiplier) + carry;
        product.push(b'0' + (value % 10) as u8);
        carry = value / 10;
    }
    while carry > 0 {
        product.push(b'0' + (carry % 10) as u8);
        carry /= 10;
    }
    product.reverse();
    product
}

fn parse_bounded_exponent(text: &str) -> Result<i32, String> {
    if text.is_empty() || text.len() > 5 {
        return Err("duration claim exponent is missing or unreasonable".into());
    }
    let exponent = text
        .parse::<i32>()
        .map_err(|_| "duration claim exponent is invalid".to_string())?;
    if exponent.unsigned_abs() > MAX_EXPONENT as u32 {
        return Err("duration claim exponent is unreasonable".into());
    }
    Ok(exponent)
}
