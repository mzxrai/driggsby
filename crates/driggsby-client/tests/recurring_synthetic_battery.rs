mod support;

use support::recurring_testkit::{recurring_group_exists, run_scenario, transaction};

#[test]
fn synthetic_battery_covers_required_recurring_matrix() {
    // monthly fixed amount (positive)
    let monthly_fixed = vec![
        transaction(
            "acct_a",
            "2026-01-10",
            -100.0,
            "USD",
            "RENT",
            Some("Landlord LLC"),
        ),
        transaction(
            "acct_a",
            "2026-02-10",
            -100.0,
            "USD",
            "RENT",
            Some("Landlord LLC"),
        ),
        transaction(
            "acct_a",
            "2026-03-10",
            -100.0,
            "USD",
            "RENT",
            Some("Landlord LLC"),
        ),
    ];
    assert!(!run_scenario(&monthly_fixed, None, None).is_empty());

    // monthly end-of-month clamp behavior (positive)
    let monthly_eom = vec![
        transaction(
            "acct_a",
            "2026-01-31",
            -80.0,
            "USD",
            "INSURANCE",
            Some("InsureCo"),
        ),
        transaction(
            "acct_a",
            "2026-02-28",
            -80.0,
            "USD",
            "INSURANCE",
            Some("InsureCo"),
        ),
        transaction(
            "acct_a",
            "2026-03-31",
            -80.0,
            "USD",
            "INSURANCE",
            Some("InsureCo"),
        ),
    ];
    assert!(!run_scenario(&monthly_eom, None, None).is_empty());

    // monthly slight amount variance (positive)
    let monthly_variance = vec![
        transaction(
            "acct_a",
            "2026-01-12",
            -98.0,
            "USD",
            "INTERNET",
            Some("FiberNet"),
        ),
        transaction(
            "acct_a",
            "2026-02-12",
            -102.0,
            "USD",
            "INTERNET",
            Some("FiberNet"),
        ),
        transaction(
            "acct_a",
            "2026-03-12",
            -101.0,
            "USD",
            "INTERNET",
            Some("FiberNet"),
        ),
    ];
    assert!(!run_scenario(&monthly_variance, None, None).is_empty());

    // weekly fixed cadence (positive)
    let weekly_fixed = vec![
        transaction(
            "acct_a",
            "2026-01-02",
            -15.0,
            "USD",
            "COFFEE CLUB",
            Some("Coffee Club"),
        ),
        transaction(
            "acct_a",
            "2026-01-09",
            -15.0,
            "USD",
            "COFFEE CLUB",
            Some("Coffee Club"),
        ),
        transaction(
            "acct_a",
            "2026-01-16",
            -15.0,
            "USD",
            "COFFEE CLUB",
            Some("Coffee Club"),
        ),
        transaction(
            "acct_a",
            "2026-01-23",
            -15.0,
            "USD",
            "COFFEE CLUB",
            Some("Coffee Club"),
        ),
    ];
    assert!(recurring_group_exists(
        &run_scenario(&weekly_fixed, None, None),
        "COFFEE CLUB",
        "weekly"
    ));

    // weekly shifted by one day once (positive)
    let weekly_shifted = vec![
        transaction(
            "acct_a",
            "2026-02-01",
            -9.0,
            "USD",
            "NEWSLETTER",
            Some("News Weekly"),
        ),
        transaction(
            "acct_a",
            "2026-02-08",
            -9.0,
            "USD",
            "NEWSLETTER",
            Some("News Weekly"),
        ),
        transaction(
            "acct_a",
            "2026-02-16",
            -9.0,
            "USD",
            "NEWSLETTER",
            Some("News Weekly"),
        ),
        transaction(
            "acct_a",
            "2026-02-23",
            -9.0,
            "USD",
            "NEWSLETTER",
            Some("News Weekly"),
        ),
    ];
    assert!(!run_scenario(&weekly_shifted, None, None).is_empty());

    // biweekly fixed cadence (positive)
    let biweekly_fixed = vec![
        transaction(
            "acct_a",
            "2026-01-03",
            -35.0,
            "USD",
            "DOG WALKER",
            Some("Dog Walker"),
        ),
        transaction(
            "acct_a",
            "2026-01-17",
            -35.0,
            "USD",
            "DOG WALKER",
            Some("Dog Walker"),
        ),
        transaction(
            "acct_a",
            "2026-01-31",
            -35.0,
            "USD",
            "DOG WALKER",
            Some("Dog Walker"),
        ),
        transaction(
            "acct_a",
            "2026-02-14",
            -35.0,
            "USD",
            "DOG WALKER",
            Some("Dog Walker"),
        ),
    ];
    assert!(recurring_group_exists(
        &run_scenario(&biweekly_fixed, None, None),
        "DOG WALKER",
        "biweekly"
    ));

    // biweekly with holiday shift (positive)
    let biweekly_shift = vec![
        transaction(
            "acct_a",
            "2026-03-01",
            -60.0,
            "USD",
            "CLEANING",
            Some("Clean Team"),
        ),
        transaction(
            "acct_a",
            "2026-03-15",
            -60.0,
            "USD",
            "CLEANING",
            Some("Clean Team"),
        ),
        transaction(
            "acct_a",
            "2026-03-30",
            -60.0,
            "USD",
            "CLEANING",
            Some("Clean Team"),
        ),
        transaction(
            "acct_a",
            "2026-04-13",
            -60.0,
            "USD",
            "CLEANING",
            Some("Clean Team"),
        ),
    ];
    assert!(!run_scenario(&biweekly_shift, None, None).is_empty());

    // merchant missing + strong description fingerprint (positive)
    let strong_desc = vec![
        transaction(
            "acct_a",
            "2026-01-04",
            -12.99,
            "USD",
            "NETFLIX MONTHLY MEMBERSHIP",
            None,
        ),
        transaction(
            "acct_a",
            "2026-02-04",
            -12.99,
            "USD",
            "NETFLIX MONTHLY MEMBERSHIP",
            None,
        ),
        transaction(
            "acct_a",
            "2026-03-04",
            -12.99,
            "USD",
            "NETFLIX MONTHLY MEMBERSHIP",
            None,
        ),
    ];
    assert!(!run_scenario(&strong_desc, None, None).is_empty());

    // merchant missing + weak generic descriptions (negative)
    let weak_desc = vec![
        transaction(
            "acct_a",
            "2026-01-05",
            -12.99,
            "USD",
            "POS DEBIT CARD PURCHASE 1111",
            None,
        ),
        transaction(
            "acct_a",
            "2026-02-05",
            -12.99,
            "USD",
            "POS DEBIT CARD PURCHASE 2222",
            None,
        ),
        transaction(
            "acct_a",
            "2026-03-05",
            -12.99,
            "USD",
            "POS DEBIT CARD PURCHASE 3333",
            None,
        ),
    ];
    assert!(run_scenario(&weak_desc, None, None).is_empty());

    // opposite sign streams for same merchant (separation expected)
    let sign_split = vec![
        transaction(
            "acct_a",
            "2026-01-08",
            -40.0,
            "USD",
            "UTILITY",
            Some("Water Co"),
        ),
        transaction(
            "acct_a",
            "2026-02-08",
            -40.0,
            "USD",
            "UTILITY",
            Some("Water Co"),
        ),
        transaction(
            "acct_a",
            "2026-03-08",
            -40.0,
            "USD",
            "UTILITY",
            Some("Water Co"),
        ),
        transaction(
            "acct_a",
            "2026-01-09",
            40.0,
            "USD",
            "UTILITY REFUND",
            Some("Water Co"),
        ),
        transaction(
            "acct_a",
            "2026-02-09",
            40.0,
            "USD",
            "UTILITY REFUND",
            Some("Water Co"),
        ),
        transaction(
            "acct_a",
            "2026-03-09",
            40.0,
            "USD",
            "UTILITY REFUND",
            Some("Water Co"),
        ),
    ];
    assert!(run_scenario(&sign_split, None, None).len() >= 2);

    // same merchant across multiple currencies (separation expected)
    let currency_split = vec![
        transaction(
            "acct_a",
            "2026-01-15",
            -20.0,
            "USD",
            "SERVICE",
            Some("Cloud App"),
        ),
        transaction(
            "acct_a",
            "2026-02-15",
            -20.0,
            "USD",
            "SERVICE",
            Some("Cloud App"),
        ),
        transaction(
            "acct_a",
            "2026-03-15",
            -20.0,
            "USD",
            "SERVICE",
            Some("Cloud App"),
        ),
        transaction(
            "acct_a",
            "2026-01-16",
            -18.0,
            "EUR",
            "SERVICE",
            Some("Cloud App"),
        ),
        transaction(
            "acct_a",
            "2026-02-16",
            -18.0,
            "EUR",
            "SERVICE",
            Some("Cloud App"),
        ),
        transaction(
            "acct_a",
            "2026-03-16",
            -18.0,
            "EUR",
            "SERVICE",
            Some("Cloud App"),
        ),
    ];
    let currency_patterns = run_scenario(&currency_split, None, None);
    assert!(currency_patterns.len() >= 2);

    // only two occurrences (negative)
    let only_two = vec![
        transaction(
            "acct_a",
            "2026-01-01",
            -50.0,
            "USD",
            "MAGAZINE",
            Some("Magazine Co"),
        ),
        transaction(
            "acct_a",
            "2026-02-01",
            -50.0,
            "USD",
            "MAGAZINE",
            Some("Magazine Co"),
        ),
    ];
    assert!(run_scenario(&only_two, None, None).is_empty());

    // high amount volatility (negative)
    let high_volatility = vec![
        transaction(
            "acct_a",
            "2026-01-10",
            -10.0,
            "USD",
            "ELECTRIC",
            Some("Grid Co"),
        ),
        transaction(
            "acct_a",
            "2026-02-10",
            -200.0,
            "USD",
            "ELECTRIC",
            Some("Grid Co"),
        ),
        transaction(
            "acct_a",
            "2026-03-10",
            -20.0,
            "USD",
            "ELECTRIC",
            Some("Grid Co"),
        ),
    ];
    assert!(run_scenario(&high_volatility, None, None).is_empty());

    // mixed frequent discretionary spend (negative)
    let discretionary = vec![
        transaction(
            "acct_a",
            "2026-01-02",
            -6.0,
            "USD",
            "COFFEE SHOP",
            Some("Coffee Spot"),
        ),
        transaction(
            "acct_a",
            "2026-01-05",
            -7.0,
            "USD",
            "COFFEE SHOP",
            Some("Coffee Spot"),
        ),
        transaction(
            "acct_a",
            "2026-01-11",
            -8.0,
            "USD",
            "COFFEE SHOP",
            Some("Coffee Spot"),
        ),
        transaction(
            "acct_a",
            "2026-01-19",
            -4.5,
            "USD",
            "COFFEE SHOP",
            Some("Coffee Spot"),
        ),
    ];
    assert!(run_scenario(&discretionary, None, None).is_empty());

    // cadence switch within one group (negative)
    let cadence_switch = vec![
        transaction(
            "acct_a",
            "2026-01-01",
            -30.0,
            "USD",
            "SERVICE",
            Some("Switch Co"),
        ),
        transaction(
            "acct_a",
            "2026-01-08",
            -30.0,
            "USD",
            "SERVICE",
            Some("Switch Co"),
        ),
        transaction(
            "acct_a",
            "2026-01-15",
            -30.0,
            "USD",
            "SERVICE",
            Some("Switch Co"),
        ),
        transaction(
            "acct_a",
            "2026-02-15",
            -30.0,
            "USD",
            "SERVICE",
            Some("Switch Co"),
        ),
    ];
    assert!(run_scenario(&cadence_switch, None, None).is_empty());

    // shuffled input order invariance
    let ordered = vec![
        transaction(
            "acct_a",
            "2026-01-20",
            -19.0,
            "USD",
            "STREAMING",
            Some("Cinema Now"),
        ),
        transaction(
            "acct_a",
            "2026-02-20",
            -19.0,
            "USD",
            "STREAMING",
            Some("Cinema Now"),
        ),
        transaction(
            "acct_a",
            "2026-03-20",
            -19.0,
            "USD",
            "STREAMING",
            Some("Cinema Now"),
        ),
    ];
    let shuffled = vec![
        transaction(
            "acct_a",
            "2026-03-20",
            -19.0,
            "USD",
            "STREAMING",
            Some("Cinema Now"),
        ),
        transaction(
            "acct_a",
            "2026-01-20",
            -19.0,
            "USD",
            "STREAMING",
            Some("Cinema Now"),
        ),
        transaction(
            "acct_a",
            "2026-02-20",
            -19.0,
            "USD",
            "STREAMING",
            Some("Cinema Now"),
        ),
    ];
    let ordered_result = run_scenario(&ordered, None, None);
    let shuffled_result = run_scenario(&shuffled, None, None);
    assert_eq!(ordered_result, shuffled_result);

    // from/to scoped window behavior
    let scoped = vec![
        transaction(
            "acct_a",
            "2025-12-01",
            -22.0,
            "USD",
            "TOOLING",
            Some("Build Tools"),
        ),
        transaction(
            "acct_a",
            "2026-01-01",
            -22.0,
            "USD",
            "TOOLING",
            Some("Build Tools"),
        ),
        transaction(
            "acct_a",
            "2026-02-01",
            -22.0,
            "USD",
            "TOOLING",
            Some("Build Tools"),
        ),
        transaction(
            "acct_a",
            "2026-03-01",
            -22.0,
            "USD",
            "TOOLING",
            Some("Build Tools"),
        ),
    ];
    assert!(!run_scenario(&scoped, None, None).is_empty());
    assert!(run_scenario(&scoped, Some("2026-02-01"), Some("2026-03-15")).is_empty());
}
