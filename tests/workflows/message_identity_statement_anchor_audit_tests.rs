use super::statement_anchor_audit;

#[test]
fn shared_quote_anchor_does_not_validate_unrelated_statements() {
    let doc = "time is zero. applied. | float latitude (degrees)";
    let anchor = doc.find(" applied. ").unwrap();
    assert_eq!(
        statement_anchor_audit(doc, "float latitude (degrees)", anchor, |_| true),
        ("statement_elsewhere_in_owner", 1, 1)
    );
    assert_eq!(
        statement_anchor_audit(doc, "not present in source", anchor, |_| true),
        ("statement_not_found", 0, 0)
    );
}

#[test]
fn exact_statement_anchor_is_distinct_from_quote_anchor() {
    let doc = "first statement\nsecond statement";
    assert_eq!(
        statement_anchor_audit(doc, "second statement", 16, |_| true).0,
        "statement_starts_at_anchor"
    );
    assert_eq!(
        statement_anchor_audit(doc, "first statement", 16, |_| false).0,
        "statement_elsewhere"
    );
}
