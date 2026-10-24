use super::*;

    #[test]
    fn unicode_html_rows_keep_utf8_boundaries() {
        let source = "<table><tr><td>A</td><td>“Alive” – status</td></tr><tr><td>B</td><td>“Beam” – mode</td></tr></table>";
        let normalized = normalize_html_tables(source);
        assert!(normalized.contains("Type: A | “Alive” – status"));
        assert!(normalized.contains("Type: B | “Beam” – mode"));
        assert_eq!(normalized.matches("Type: ").count(), 2);
    }
    #[test]
    fn table_23_type_cells_establish_message_identity() {
        let source = "2.3 Message Types, Sender (M)\n<table><tr><td>Type</td><td>Description</td><td>No. of Parameter(s)</td><td>Name of the Parameters</td></tr><tr><td>A</td><td>Alive intervalAntenna should send a status message at least this often. 0 means never repeat.</td><td>1</td><td>int interval, seconds</td></tr></table>";
        let normalized = normalize_html_tables(source);
        assert!(normalized.contains("Type: A | Alive interval"));
        assert!(declared_message_identity("messages/a.md", &normalized));
        assert!(!declared_message_identity("messages/b.md", &normalized));
        assert!(!normalized.contains("<table"));
    }

    #[test]
    fn table_24_type_cells_and_rowspan_continuations() {
        let source = "2.4 Message Types, Receiver (A)\n<table><tr><td rowspan=\"3\">B</td><td>Beat frequency oscillator</td><td>2(minimum)</td></tr><tr><td colspan=\"2\">Additional LO frequencies</td></tr></table>";
        let normalized = normalize_html_tables(source);
        assert!(normalized.contains("Type: B | Beat frequency oscillator"));
        assert!(normalized.contains("Additional LO frequencies"));
        assert_eq!(normalized.matches("Type: B").count(), 1);
    }

    #[test]
    fn examples_28_are_not_identity_declarations() {
        let source =
            "2.8 Examples\n```txt\nA 10 # Alive\nB 18000.500 28000.500 rx2=18200\nL 1 1\n```\n";
        assert!(!declared_message_identity("messages/a.md", source));
        assert!(!declared_message_identity("messages/b.md", source));
        assert!(!declared_message_identity("messages/l.md", source));
    }

    #[test]
    fn embedded_image_is_masked_before_normalization() {
        let original = format!(
            "![](data:image/jpeg;base64,{})\n<table><tr><td>q</td><td>Query</td></tr></table>",
            "A".repeat(200)
        );
        let masked = mask_embedded_data_uris(&original);
        let normalized = normalize_html_tables(&mask_opaque_lines(&masked));
        assert!(!normalized.contains(&"A".repeat(200)));
        assert!(normalized.contains("Type: q | Query"));
        assert!(original.contains(&"A".repeat(200)));
    }

    #[test]
    fn segment_ids_reference_normalized_table_row() {
        let source =
            normalize_html_tables("<table><tr><td>s</td><td>Status message</td></tr></table>");
        let segments = evidence_segments(&source);
        assert_eq!(segments[0].id, "S1");
        assert!(declared_message_identity(
            "messages/s.md",
            &segments[0].text
        ));
    }
