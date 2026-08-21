#[cfg(test)]
mod tests {
    use aiagents::utils::jsonutils::get_json_field;
    use serde_json::json;

    #[test]
    fn returns_string_field() {
        let payload = json!({
            "name": "Alice"
        });

        let result = get_json_field(&payload, "name");

        assert_eq!(result, Ok("Alice".to_string()));
    }

    #[test]
    fn returns_error_when_field_is_missing() {
        let payload = json!({
            "name": "Alice"
        });

        let result = get_json_field(&payload, "age");

        assert_eq!(
            result,
            Err(r#"field age not found:
{"name":"Alice"}"#.to_string())
        );
    }

    #[test]
    fn returns_error_when_field_is_not_a_string() {
        let payload = json!({
            "age": 42
        });

        let result = get_json_field(&payload, "age");

        assert_eq!(
            result,
            Err(r#"got field age, but it is empty:
{"age":42}"#.to_string())
        );
    }

    #[test]
    fn returns_error_when_field_is_null() {
        let payload = json!({
            "name": null
        });

        let result = get_json_field(&payload, "name");

        assert_eq!(
            result,
            Err(r#"got field name, but it is empty:
{"name":null}"#.to_string())
        );
    }

    #[test]
    fn returns_empty_string_for_empty_string_field() {
        let payload = json!({
            "name": ""
        });

        let result = get_json_field(&payload, "name");

        assert_eq!(result, Ok("".to_string()));
    }
}

