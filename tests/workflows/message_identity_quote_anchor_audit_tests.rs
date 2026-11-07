use super::quote_anchor_audit;

#[test]
fn distinguishes_exact_overlap_and_disjoint() {
    let doc = "abc\nmulti\nline\nother";
    assert_eq!(
        quote_anchor_audit(doc, "multi\nline", 4).0,
        "quote_starts_at_anchor"
    );
    assert_eq!(
        quote_anchor_audit(doc, "multi\nline", 8).0,
        "quote_overlaps_anchor"
    );
    assert_eq!(
        quote_anchor_audit(doc, "other", 4).0,
        "quote_disjoint_from_anchor"
    );
}

#[test]
fn short_repeated_quotes_are_reported_as_ambiguous() {
    assert_eq!(
        quote_anchor_audit("x applied. y applied. ", " applied. ", 0).1,
        2
    );
    assert_eq!(
        quote_anchor_audit("abc", "", 0).0,
        "missing_quote_or_anchor"
    );
}
