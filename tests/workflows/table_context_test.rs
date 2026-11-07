use super::*;

#[test]
fn context_contains_only_rows_inside_window() {
    let src = "Before\n| Name | Value |\n| --- | --- |\n| A | 1 |\n| B | 2 |\nAfter\n";
    let analysis = DocumentAnalysis::parse(src);
    let start = src.find("| A |").unwrap();
    let end = src.find("| B |").unwrap();
    let context = table_context(&analysis, start, end);
    assert!(context.contains("A"));
    assert!(context.contains("kind=Body"));
    assert!(!context.contains("text=\"B\""));
    assert!(table_context(&analysis, 0, 6).is_empty());
}

#[test]
fn html_cells_keep_explicit_spans() {
    let src = concat!(
        "<table><tr><th>Type</th><th>Value</th></tr>",
        "<tr><td rowspan=\"2\">A</td><td>1</td></tr>",
        "<tr><td>2</td></tr></table>"
    );
    let analysis = DocumentAnalysis::parse(src);
    let context = table_context(&analysis, 0, src.len());
    assert!(context.contains("syntax=HTML"));
    assert!(context.contains("rowspan=2"));
    assert!(context.contains("columns=2"));
}

#[test]
fn context_does_not_infer_flattened_tables() {
    let src = "name | value\nA | 1\nB | 2\n";
    let analysis = DocumentAnalysis::parse(src);
    assert!(table_context(&analysis, 0, src.len()).is_empty());
}
