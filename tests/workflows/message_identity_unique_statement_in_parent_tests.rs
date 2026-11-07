use super::unique_statement_in_parent;

#[test]
fn exact_statement_has_independent_parent_offset() {
    assert_eq!(
        unique_statement_in_parent("alpha | beta", "beta", Some(20)),
        Some(28)
    );
    assert_eq!(
        unique_statement_in_parent("alpha | beta", "alpha", Some(20)),
        Some(20)
    );
}

#[test]
fn ambiguous_missing_or_unplaced_statement_is_not_located() {
    assert_eq!(
        unique_statement_in_parent("beta | beta", "beta", Some(20)),
        None
    );
    assert_eq!(
        unique_statement_in_parent("alpha | beta", "paraphrase", Some(20)),
        None
    );
    assert_eq!(
        unique_statement_in_parent("alpha | beta", "beta", None),
        None
    );
    assert_eq!(
        unique_statement_in_parent("alpha", "alpha", Some(usize::MAX)),
        Some(usize::MAX)
    );
    assert_eq!(
        unique_statement_in_parent("xalpha", "alpha", Some(usize::MAX)),
        None
    );
}
