use super::statement_window_relation;

#[test]
fn requires_entire_statement_inside_window() {
    assert_eq!(
        statement_window_relation(Some(110), 10, 100, 30),
        "inside_window"
    );
    assert_eq!(
        statement_window_relation(Some(125), 10, 100, 30),
        "crosses_window_boundary"
    );
    assert_eq!(
        statement_window_relation(Some(95), 10, 100, 30),
        "crosses_window_boundary"
    );
    assert_eq!(
        statement_window_relation(Some(80), 10, 100, 30),
        "outside_window"
    );
    assert_eq!(
        statement_window_relation(None, 10, 100, 30),
        "no_unique_statement"
    );
}

#[test]
fn boundary_and_overflow_are_conservative() {
    assert_eq!(
        statement_window_relation(Some(120), 10, 100, 30),
        "inside_window"
    );
    assert_eq!(
        statement_window_relation(Some(130), 1, 100, 30),
        "outside_window"
    );
    assert_eq!(
        statement_window_relation(Some(usize::MAX), 2, 100, 30),
        "outside_window"
    );
}
