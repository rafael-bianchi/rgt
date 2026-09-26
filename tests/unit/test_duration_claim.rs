use rgt::types::{DurationClaim, DurationUnit};

#[test]
fn equivalent_claims_in_all_fixed_units_have_the_same_seconds() {
    let claims = [
        ("604800", "seconds"),
        ("10080", "minutes"),
        ("168", "hours"),
        ("7", "days"),
        ("1", "weeks"),
    ];
    for (lexeme, unit) in claims {
        let parsed = DurationClaim::parse(lexeme, DurationUnit::parse(unit).unwrap()).unwrap();
        assert_eq!(parsed.exact_seconds(), 604_800, "{lexeme} {unit}");
    }
}

#[test]
fn signed_decimal_and_scientific_claim_syntax_is_exact() {
    assert_eq!(
        DurationClaim::parse("+45.0", DurationUnit::Days)
            .unwrap()
            .exact_seconds(),
        3_888_000
    );
    assert_eq!(
        DurationClaim::parse("4.5e1", DurationUnit::Days)
            .unwrap()
            .exact_seconds(),
        3_888_000
    );
    assert_eq!(
        DurationClaim::parse(".1", DurationUnit::Minutes)
            .unwrap()
            .exact_seconds(),
        6
    );
    assert_eq!(
        DurationClaim::parse("-1.5", DurationUnit::Hours)
            .unwrap()
            .exact_seconds(),
        -5_400
    );
}

#[test]
fn rejects_decimal_claims_that_need_fractional_seconds_or_exceed_range() {
    assert!(DurationClaim::parse("0.01", DurationUnit::Minutes).is_err());
    assert!(DurationClaim::parse("1.0000000000000001", DurationUnit::Seconds).is_err());
    assert!(DurationClaim::parse("1e999999", DurationUnit::Seconds).is_err());
}

#[test]
fn accepts_exact_claims_when_the_intermediate_mantissa_exceeds_i128() {
    let trailing_zero_fraction = DurationClaim::parse(
        "1.0000000000000000000000000000000000000000",
        DurationUnit::Seconds,
    )
    .unwrap();
    assert_eq!(trailing_zero_fraction.exact_seconds(), 1);

    let large_significand = format!("6{}e-38", "0".repeat(38));
    let compensated_exponent =
        DurationClaim::parse(&large_significand, DurationUnit::Seconds).unwrap();
    assert_eq!(compensated_exponent.exact_seconds(), 6);

    let large_fractional = format!("6{}1e-39", "0".repeat(38));
    assert!(DurationClaim::parse(&large_fractional, DurationUnit::Seconds).is_err());
}
