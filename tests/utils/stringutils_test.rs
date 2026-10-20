#[cfg(test)]
mod tests {
    use aiagents::utils::stringutils::extract_known_paths;
    use aiagents::utils::stringutils::extract_standalone_choice;
    use aiagents::utils::stringutils::iter_fenced_blocks;
    use aiagents::utils::stringutils::raw_fence_to_string;
    use aiagents::utils::stringutils::resolve_source_quote;
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
        let input =
            "```rust\nnot the preferred block\n```\n\n```json\n{\n  \"name\": \"test\"\n}\n```";

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
        assert_eq!(
            strip_outer_markdown_fence(input),
            "cpp/src/OpenResult.hpp\ncpp/src/Shmem.hpp"
        );
    }

    #[test]
    fn strip_outer_markdown_fence_returns_raw_text_without_fence() {
        let input = "  cpp/src/OpenResult.hpp\ncpp/src/Shmem.hpp  ";
        assert_eq!(
            strip_outer_markdown_fence(input),
            "cpp/src/OpenResult.hpp\ncpp/src/Shmem.hpp"
        );
    }

    #[test]
    fn extract_known_paths_handles_llm_formatting() {
        let known = vec![
            "cpp/src/OpenResult.hpp".into(),
            "cpp/src/Shmem.cpp".into(),
            "cpp/src/Shmem.hpp".into(),
        ];
        let input = "The relevant files are:\n1. `/home/witold/project/cpp/src/Shmem.cpp` - where the error occurs\n2. `/home/witold/project/cpp/src/OpenResult.hpp` - where the result type is defined";

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
        let known = vec!["src/OpenResult.hpp".into(), "cpp/src/OpenResult.hpp".into()];
        let input = "`/project/cpp/src/OpenResult.hpp`\ncpp/src/OpenResult.hpp";

        assert_eq!(
            extract_known_paths(input, &known, 2),
            vec![std::path::PathBuf::from("cpp/src/OpenResult.hpp")]
        );
    }

    #[test]
    fn extract_standalone_choice_accepts_plain_marked_and_design_prefixed_numbers() {
        assert_eq!(extract_standalone_choice("2", 3), Some(2));
        assert_eq!(extract_standalone_choice("**3**", 3), Some(3));
        assert_eq!(extract_standalone_choice("`1`", 3), Some(1));
        assert_eq!(extract_standalone_choice("Design 2", 3), Some(2));
        assert_eq!(extract_standalone_choice("DESIGN 1", 3), Some(1));
        assert_eq!(extract_standalone_choice("**Design 3**", 3), Some(3));
    }

    #[test]
    fn extract_standalone_choice_rejects_out_of_range_and_explanatory_prose() {
        assert_eq!(extract_standalone_choice("4", 3), None);
        assert_eq!(extract_standalone_choice("Design 4", 3), None);
        assert_eq!(extract_standalone_choice("I prefer Design 2", 3), None);
        assert_eq!(
            extract_standalone_choice("Design 2 because it is simpler", 3),
            None
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

        assert_eq!(
            result,
            "{\"first\": \"hello\\n\", \"second\": \"world\\n\"}"
        );
    }

    #[test]
    fn source_quote_allows_whitespace_only_changes_and_preserves_source_bytes() {
        let source = "The timeout is\n  exactly   thirty seconds. Another line.";
        let quote = "timeout is exactly thirty seconds";
        assert_eq!(
            resolve_source_quote(source, quote).as_deref(),
            Some("timeout is\n  exactly   thirty seconds")
        );
        assert!(resolve_source_quote(source, "timeout is exactly forty seconds").is_none());
        assert!(resolve_source_quote(source, "Timeout is exactly thirty seconds").is_none());
        assert!(resolve_source_quote(source, "short").is_none());
        assert_eq!(
            resolve_source_quote(
                "The command uses <lf> and <cr><lf> sequences.",
                "command uses &lt;lf&gt; and &lt;cr&gt;&lt;lf&gt; sequences"
            )
            .as_deref(),
            Some("command uses <lf> and <cr><lf> sequences")
        );
        assert_eq!(
            resolve_source_quote(
                "The command uses &lt;lf&gt; and &lt;cr&gt;&lt;lf&gt; sequences.",
                "command uses <lf> and <cr><lf> sequences"
            )
            .as_deref(),
            Some("command uses &lt;lf&gt; and &lt;cr&gt;&lt;lf&gt; sequences")
        );
        assert!(
            resolve_source_quote(
                "The command uses <lf> and <cr><lf> sequences.",
                "command uses <lf> and <xx><lf> sequences"
            )
            .is_none()
        );
        let unicode = "Änderung\t  beträgt zwölf Sekunden.";
        assert_eq!(
            resolve_source_quote(unicode, "Änderung beträgt zwölf Sekunden.").as_deref(),
            Some(unicode)
        );
    }
    #[test]
    fn quote_rejects_empty_and_whitespace_only_input() {
        assert_eq!(resolve_source_quote("a source sentence", ""), None);
        assert_eq!(
            resolve_source_quote("a source sentence", "             "),
            None
        );
        assert_eq!(
            resolve_source_quote("a source sentence", "\n\t           "),
            None
        );
        assert_eq!(resolve_source_quote("", "a sufficiently long quote"), None);
    }

    #[test]
    fn quote_preserves_exact_source_and_byte_boundaries() {
        let source = "Prefix: Änderung\n\t beträgt zwölf Sekunden. Suffix.";
        assert_eq!(
            resolve_source_quote(source, "Änderung beträgt zwölf Sekunden.").as_deref(),
            Some("Änderung\n\t beträgt zwölf Sekunden.")
        );
        assert_eq!(
            resolve_source_quote(source, "Änderung\n\t beträgt zwölf Sekunden.").as_deref(),
            Some("Änderung\n\t beträgt zwölf Sekunden.")
        );
    }

    #[test]
    fn quote_handles_html_entity_spellings_and_original_source() {
        let source = "Command <lf> & <cr><lf> end.";
        for quote in [
            "Command &lt;lf&gt; &amp; &lt;cr&gt;&lt;lf&gt; end.",
            "Command &#60;lf&#62; &#38; &#60;cr&#62;&#60;lf&#62; end.",
            "Command &#x3c;lf&#x3e; &#x26; &#x3c;cr&#x3e;&#x3c;lf&#x3e; end.",
        ] {
            assert_eq!(resolve_source_quote(source, quote).as_deref(), Some(source));
        }
        let escaped_source = "Command &lt;lf&gt; &amp; &lt;cr&gt;&lt;lf&gt; end.";
        assert_eq!(
            resolve_source_quote(escaped_source, source).as_deref(),
            Some(escaped_source)
        );
    }

    #[test]
    fn quote_rejects_changes_other_than_whitespace_or_supported_entities() {
        let source = "Timeout is exactly thirty seconds; use <lf>.";
        for quote in [
            "Timeout is exactly forty seconds; use <lf>.",
            "timeout is exactly thirty seconds; use <lf>.",
            "Timeout is exactly thirty seconds, use <lf>.",
            "Timeout is exactly thirty seconds; use <cr>.",
            "Timeout is exactly thirty seconds; use &quot;lf&quot;.",
        ] {
            assert_eq!(resolve_source_quote(source, quote), None, "{quote}");
        }
    }

    #[test]
    fn quote_matches_across_mixed_whitespace_and_entities() {
        let source = "One\r\n  <lf>\t and\n<cr><lf> terminators.";
        assert_eq!(
            resolve_source_quote(
                source,
                "One &lt;lf&gt; and &lt;cr&gt;&lt;lf&gt; terminators."
            )
            .as_deref(),
            Some(source)
        );
    }
}
