#[cfg(test)]
mod tests {
    use aiagents::utils::stringutils::extract_known_paths;
    use aiagents::utils::stringutils::iter_fenced_blocks;
    use aiagents::utils::stringutils::raw_fence_to_string;
    use aiagents::utils::stringutils::strip_code_fences;
    use aiagents::utils::stringutils::strip_outer_markdown_fence;


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
    fn strip_outer_markdown_fence_preserves_first_plaintext_line() {
        let input = "```cpp/src/OpenResult.hpp\ncpp/src/Shmem.hpp\n```";
        assert_eq!(strip_outer_markdown_fence(input), "cpp/src/OpenResult.hpp\ncpp/src/Shmem.hpp");
    }

    #[test]
    fn strip_outer_markdown_fence_returns_raw_text_without_fence() {
        let input = "  cpp/src/OpenResult.hpp\ncpp/src/Shmem.hpp  ";
        assert_eq!(strip_outer_markdown_fence(input), "cpp/src/OpenResult.hpp\ncpp/src/Shmem.hpp");
    }

    #[test]
    fn extract_known_paths_handles_llm_formatting() {
        let known = vec![
            "cpp/src/OpenResult.hpp".into(),
            "cpp/src/Shmem.cpp".into(),
            "cpp/src/Shmem.hpp".into(),
        ];
        let input = "The relevant files are:\\n1. `/home/witold/project/cpp/src/Shmem.cpp` - where the error occurs\\n2. `/home/witold/project/cpp/src/OpenResult.hpp` - where the result type is defined";

        assert_eq!(
            extract_known_paths(input, &known, 2),
            vec![
                std::path::PathBuf::from("cpp/src/Shmem.cpp"),
                std::path::PathBuf::from("cpp/src/OpenResult.hpp"),
            ]
        );
    }

    #[test]
    fn extract_known_paths_prefers_longest_match_and_ignores_duplicates() {
        let known = vec![
            "src/OpenResult.hpp".into(),
            "cpp/src/OpenResult.hpp".into(),
        ];
        let input = "`/project/cpp/src/OpenResult.hpp`\\ncpp/src/OpenResult.hpp";

        assert_eq!(
            extract_known_paths(input, &known, 2),
            vec![std::path::PathBuf::from("cpp/src/OpenResult.hpp")]
        );
    }

    #[test]
    fn raw_fence_to_string_does_not_loop_on_missing_end_marker() {
        let input = "prefix RAW_TEXT_BEGIN>>\nhello";

        let result = raw_fence_to_string(input);

        assert_eq!(result, input);
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
    fn raw_fence_to_string_preserves_existing_comma_between_multiple_raw_values() {
        let input = "{\"first\": RAW_TEXT_BEGIN>>\nhello\nRAW_TEXT_END, \"second\": RAW_TEXT_BEGIN>>\nworld\nRAW_TEXT_END}";

        let result = raw_fence_to_string(input);

        assert_eq!(result, "{\"first\": \"hello\\n\", \"second\": \"world\\n\"}");
    }
}

