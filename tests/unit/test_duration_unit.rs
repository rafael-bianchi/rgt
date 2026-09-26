use rgt::types::{DurationClaim, DurationUnit};

#[test]
fn parses_only_the_five_fixed_lowercase_units() {
    let cases = [
        ("seconds", 1),
        ("minutes", 60),
        ("hours", 3_600),
        ("days", 86_400),
        ("weeks", 604_800),
    ];
    for (name, scale) in cases {
        let unit = DurationUnit::parse(name).unwrap();
        assert_eq!(unit.name(), name);
        assert_eq!(unit.seconds_per_unit(), scale);
    }
    for invalid in ["Seconds", "day", "months", "years", " mins", ""] {
        assert!(
            DurationUnit::parse(invalid).is_err(),
            "accepted {invalid:?}"
        );
    }
}

#[test]
fn claims_convert_exact_decimal_values_without_floating_point_rounding() {
    let claim = DurationClaim::parse("0.1", DurationUnit::Minutes).unwrap();
    assert_eq!(claim.lexeme(), "0.1");
    assert_eq!(claim.exact_seconds(), 6);

    let signed = DurationClaim::parse("-45.5", DurationUnit::Minutes).unwrap();
    assert_eq!(signed.exact_seconds(), -2_730);

    assert!(DurationClaim::parse("0.01", DurationUnit::Minutes).is_err());
    assert!(DurationClaim::parse("1.0000000000000001", DurationUnit::Seconds).is_err());
}

#[test]
fn rejects_malformed_unbounded_and_unrepresentable_claims() {
    for invalid in ["NaN", "inf", "1e999999", "--1", "", "1.2.3"] {
        assert!(
            DurationClaim::parse(invalid, DurationUnit::Seconds).is_err(),
            "accepted {invalid:?}"
        );
    }
    assert!(DurationClaim::parse("9223372036854775807", DurationUnit::Seconds).is_err());
    assert!(DurationClaim::parse(&"1".repeat(140), DurationUnit::Seconds).is_err());
}

#[test]
fn selected_display_is_exact_when_terminating_and_marked_when_repeating() {
    let exact = DurationUnit::Days.display(3_888_000);
    assert_eq!(exact.decimal, "45");
    assert!(!exact.approximate);
    assert_eq!(exact.text, "45 days");

    let repeating = DurationUnit::Minutes.display(1);
    assert_eq!(repeating.decimal, "0.016666666667");
    assert!(repeating.approximate);
    assert!(repeating.text.contains("approximately"));
    assert_eq!(repeating.duration_seconds, "1");

    let negative = DurationUnit::Hours.display(-3_600);
    assert_eq!(negative.decimal, "-1");
    assert!(!negative.approximate);
}
