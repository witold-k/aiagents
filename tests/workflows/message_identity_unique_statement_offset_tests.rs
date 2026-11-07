use super::unique_statement_offset_in_owner;

#[test]
fn unique_statement_is_located_independently_of_shared_quote() {
    let doc = "applied. | float latitude | float longitude";
    let at = doc.find("float longitude").unwrap();
    assert_eq!(
        unique_statement_offset_in_owner(doc, "float longitude", |_| true),
        Some(at)
    );
}

#[test]
fn repeated_statement_or_unrelated_owner_is_not_located() {
    let doc = "latitude | latitude | longitude";
    assert_eq!(
        unique_statement_offset_in_owner(doc, "latitude", |_| true),
        None
    );
    assert_eq!(
        unique_statement_offset_in_owner(doc, "longitude", |_| false),
        None
    );
    assert_eq!(
        unique_statement_offset_in_owner(doc, "paraphrased longitude", |_| true),
        None
    );
}
