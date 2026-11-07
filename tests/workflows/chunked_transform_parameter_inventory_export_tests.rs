use super::*;

#[test]
fn exports_distinct_assertions_without_overwriting() {
    let dir = std::env::temp_dir().join(format!(
        "aifix-parameter-inventory-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("parameters-unverified.json");
    let facts = vec![
        message_identity::ParameterFact {
            statement: "trgSatPos is the target longitude".into(),
            parameter: "trgSatPos".into(),
            section: "## A.1.3 Configuration".into(),
            evidence_quote: "| trgSatPos | longitude | float |\n".into(),
            source_byte_offset: 154593,
        },
        message_identity::ParameterFact {
            statement: "trgSatPos is a float".into(),
            parameter: "trgSatPos".into(),
            section: "## A.1.3 Configuration".into(),
            evidence_quote: "| trgSatPos | longitude | float |\n".into(),
            source_byte_offset: 154593,
        },
    ];
    export_parameter_inventory(&path, &facts).unwrap();
    let decoded: Vec<message_identity::ParameterFact> =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(decoded, facts);
    assert!(export_parameter_inventory(&path, &facts).is_err());
    assert_eq!(
        fs::read(&path).unwrap(),
        serde_json::to_vec_pretty(&facts)
            .unwrap()
            .into_iter()
            .chain([b'\n'])
            .collect::<Vec<_>>()
    );
    fs::remove_dir_all(dir).unwrap();
}
