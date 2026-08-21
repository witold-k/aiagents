#[cfg(test)]
mod tests {
    use aiagents::utils::stringutils::iter_fenced_blocks;
    use aiagents::utils::stringutils::raw_fence_to_string;
    use aiagents::utils::stringutils::strip_code_fences;


    #[test]
    fn iter_fenced_blocks_finds_single_block_without_language() {
        let input = "before\n```\nhello\n```\nafter";

        let blocks = iter_fenced_blocks(input);

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].lang, None);
        assert_eq!(blocks[0].content, "hello\n");
    }

    #[test]
    fn iter_fenced_blocks_extracts_language() {
        let input = "```rust\nfn main() {}\n```";

        let blocks = iter_fenced_blocks(input);

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].lang, Some("rust"));
        assert_eq!(blocks[0].content, "fn main() {}\n");
    }

    #[test]
    fn iter_fenced_blocks_trims_language() {
        let input = "```  rust  \nfn main() {}\n```";

        let blocks = iter_fenced_blocks(input);

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].lang, Some("rust"));
    }

    #[test]
    fn iter_fenced_blocks_finds_multiple_blocks() {
        let input = "text\n```rust\nfn main() {}\n```\n\n```json\n{\"key\": \"value\"}\n```\n";

        let blocks = iter_fenced_blocks(input);

        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].lang, Some("rust"));
        assert_eq!(blocks[0].content, "fn main() {}\n");
        assert_eq!(blocks[1].lang, Some("json"));
        assert_eq!(blocks[1].content, "{\"key\": \"value\"}\n");
    }

    #[test]
    fn iter_fenced_blocks_handles_empty_language() {
        let input = "```\ncontent\n```";

        let blocks = iter_fenced_blocks(input);

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].lang, None);
    }

    #[test]
    fn iter_fenced_blocks_handles_whitespace_only_language() {
        let input = "```   \ncontent\n```";

        let blocks = iter_fenced_blocks(input);

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].lang, None);
    }

    #[test]
    fn iter_fenced_blocks_ignores_unclosed_block() {
        let input = "```rust\nfn main() {}";

        let blocks = iter_fenced_blocks(input);

        assert!(blocks.is_empty());
    }

    #[test]
    fn iter_fenced_blocks_ignores_opening_fence_without_newline() {
        let input = "```rust";

        let blocks = iter_fenced_blocks(input);

        assert!(blocks.is_empty());
    }

    #[test]
    fn iter_fenced_blocks_returns_empty_for_plain_text() {
        let blocks = iter_fenced_blocks("hello world");

        assert!(blocks.is_empty());
    }

    #[test]
    fn strip_code_fences_prefers_json() {
        let input = "```rust\nnot the preferred block\n```\n\n```json\n{\n  \"name\": \"test\"\n}\n```";

        let result = strip_code_fences(input);

        assert_eq!(result, "{\n  \"name\": \"test\"\n}");
    }

    #[test]
    fn strip_code_fences_uses_first_block_when_no_json_exists() {
        let input = "```rust\nfn main() {}\n```\n\n```python\nprint('hello')\n```";

        let result = strip_code_fences(input);

        assert_eq!(result, "fn main() {}");
    }

    #[test]
    fn strip_code_fences_returns_raw_text_without_fence() {
        let input = "  hello world  ";

        let result = strip_code_fences(input);

        assert_eq!(result, "hello world");
    }

    #[test]
    fn strip_code_fences_trims_block_content() {
        let input = "```text\n\n  hello world  \n\n```";

        let result = strip_code_fences(input);

        assert_eq!(result, "hello world");
    }

    #[test]
    fn strip_code_fences_prefers_exact_json_language() {
        let input = "``` json\nfirst\n```\n\n```json\nsecond\n```";

        let result = strip_code_fences(input);

        assert_eq!(result, "second");
    }

    #[test]
    fn raw_fence_to_string_converts_single_raw_block() {
        let input = "RAW_TEXT_BEGIN>>\nhello world\nRAW_TEXT_END}";

        let result = raw_fence_to_string(input);

        assert_eq!(result, "\"hello world\"");
    }

    #[test]
    fn raw_fence_to_string_converts_raw_block_with_closing_syntax() {
        let input = "RAW_TEXT_BEGIN>>\nhello world\n</RAW_TEXT_END>";

        let result = raw_fence_to_string(input);

        assert_eq!(result, "\"hello world\"");
    }

    #[test]
    fn raw_fence_to_string_serializes_json_special_characters() {
        let input = "RAW_TEXT_BEGIN>>\n{\"foo\": \"bar\"}\nRAW_TEXT_END}";

        let result = raw_fence_to_string(input);

        assert_eq!(result, "\"{\\\"foo\\\": \\\"bar\\\"}\"");
    }

    #[test]
    fn raw_fence_to_string_escapes_newlines() {
        let input = "RAW_TEXT_BEGIN>>\nline one\nline two\nRAW_TEXT_END}";

        let result = raw_fence_to_string(input);

        assert_eq!(result, "\"line one\\nline two\\n\"");
    }

    #[test]
    fn raw_fence_to_string_handles_multiple_blocks() {
        let input = "{\n\"a\": RAW_TEXT_BEGIN>>\nhello\nRAW_TEXT_END\n\"b\": RAW_TEXT_BEGIN>>\nworld\nRAW_TEXT_END\n}";

        let result = raw_fence_to_string(input);

        assert_eq!(result, "{\n\"a\": \"hello\\n\",\n\"b\": \"world\\n\"\n}");
    }

    #[test]
    fn raw_fence_to_string_preserves_content_before_block() {
        let input = "prefix RAW_TEXT_BEGIN>>\nhello\nRAW_TEXT_END suffix";

        let result = raw_fence_to_string(input);

        assert_eq!(result, "prefix \"hello\\n\", suffix");
    }

    #[test]
    fn raw_fence_to_string_does_not_loop_on_missing_end_marker() {
        let input = "prefix RAW_TEXT_BEGIN>>\nhello";

        let result = raw_fence_to_string(input);

        assert_eq!(result, input);
    }

    #[test]
    fn raw_fence_to_string_handles_empty_content() {
        let input = "RAW_TEXT_BEGIN>>\nRAW_TEXT_END}";

        let result = raw_fence_to_string(input);

        assert_eq!(result, "\"\"");
    }

    #[test]
    fn raw_fence_to_string_inserts_comma_between_json_fields() {
        let input = "{\"first\": RAW_TEXT_BEGIN>>\nhello\nRAW_TEXT_END \"second\": \"value\"}";

        let result = raw_fence_to_string(input);

        assert_eq!(result, "{\"first\": \"hello\\n\", \"second\": \"value\"}");
    }

    #[test]
    fn raw_fence_to_string_preserves_existing_comma() {
        let input = "{\"first\": RAW_TEXT_BEGIN>>\nhello\nRAW_TEXT_END, \"second\": \"value\"}";

        let result = raw_fence_to_string(input);

        assert_eq!(result, "{\"first\": \"hello\\n\", \"second\": \"value\"}");
    }

    #[test]
    fn raw_fence_to_string_does_not_add_comma_before_closing_brace() {
        let input = "{\"first\": RAW_TEXT_BEGIN>>\nhello\nRAW_TEXT_END}";

        let result = raw_fence_to_string(input);

        assert_eq!(result, "{\"first\": \"hello\\n\"}");
    }

    #[test]
    fn raw_fence_to_string_inserts_comma_for_multiple_raw_values() {
        let input = "{\"first\": RAW_TEXT_BEGIN>>\nhello\nRAW_TEXT_END \"second\": RAW_TEXT_BEGIN>>\nworld\nRAW_TEXT_END}";

        let result = raw_fence_to_string(input);

        assert_eq!(result, "{\"first\": \"hello\\n\", \"second\": \"world\\n\"}");
    }

    #[test]
    fn raw_fence_to_string_preserves_existing_comma_between_multiple_raw_values() {
        let input = "{\"first\": RAW_TEXT_BEGIN>>\nhello\nRAW_TEXT_END, \"second\": RAW_TEXT_BEGIN>>\nworld\nRAW_TEXT_END}";

        let result = raw_fence_to_string(input);

        assert_eq!(result, "{\"first\": \"hello\\n\", \"second\": \"world\\n\"}");
    }
}

