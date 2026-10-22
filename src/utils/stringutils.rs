// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

#[derive(Debug)]
pub struct FencedCodeBlock<'a> {
    pub lang: Option<&'a str>,
    pub content: &'a str,
}

pub fn iter_fenced_blocks<'a>(input: &'a str) -> Vec<FencedCodeBlock<'a>> {
    let mut blocks = Vec::new();
    let mut i = 0;

    while let Some(start) = input[i..].find("```") {
        let start = i + start;

        // Find end of opening line
        let line_end = match input[start + 3..].find('\n') {
            Some(v) => start + 3 + v,
            None => break,
        };

        // Extract language tag
        let lang_raw = &input[start + 3..line_end];
        let lang = if lang_raw.trim().is_empty() {
            None
        } else {
            Some(lang_raw.trim())
        };

        // Find closing ```
        let end = match input[line_end..].find("```") {
            Some(v) => line_end + v,
            None => break,
        };

        let content = &input[line_end + 1..end];

        blocks.push(FencedCodeBlock { lang, content });

        i = end + 3;
    }

    blocks
}

pub fn strip_code_fences(input: &str) -> String {
    let blocks = iter_fenced_blocks(input);

    // 1. Prefer JSON block
    if let Some(json) = blocks.iter().find(|b| b.lang == Some("json")) {
        return json.content.trim().to_string();
    }

    // 2. Otherwise first fenced block
    if let Some(first) = blocks.first() {
        return first.content.trim().to_string();
    }

    // 3. A truncated fenced response may have no closing fence. Strip only
    // the opening fence; leave the JSON itself untouched for strict parsing.
    let trimmed = input.trim();
    if let Some(rest) = trimmed.strip_prefix("```")
        && let Some((language, content)) = rest.split_once('\n')
        && (language.trim().is_empty() || language.trim().eq_ignore_ascii_case("json"))
    {
        return content.trim().to_string();
    }

    // 4. Otherwise raw text
    trimmed.to_string()
}

/// Match a model quotation against the source without changing punctuation,
/// case, or word content. Whitespace runs may differ; return original bytes.
pub fn resolve_source_quote(source: &str, quote: &str) -> Option<String> {
    let quote = quote.trim();
    if quote.chars().count() < 12 {
        return None;
    }
    if source.contains(quote) {
        return Some(quote.to_owned());
    }

    fn collapsed(text: &str) -> (String, Vec<(usize, usize)>) {
        let mut normalized = String::new();
        let mut offsets = Vec::new();
        let mut whitespace_start = None;
        let mut position = 0;
        while position < text.len() {
            let tail = &text[position..];
            let entity = [
                ("&lt;", '<'),
                ("&gt;", '>'),
                ("&amp;", '&'),
                ("&#60;", '<'),
                ("&#62;", '>'),
                ("&#38;", '&'),
                ("&#x3c;", '<'),
                ("&#x3e;", '>'),
                ("&#x26;", '&'),
            ]
            .into_iter()
            .find(|(encoded, _)| tail.starts_with(encoded));
            let (ch, length) = if let Some((encoded, decoded)) = entity {
                (decoded, encoded.len())
            } else {
                let ch = tail.chars().next().expect("nonempty tail");
                (ch, ch.len_utf8())
            };
            if ch.is_whitespace() {
                if !normalized.is_empty() && whitespace_start.is_none() {
                    whitespace_start = Some(position);
                }
            } else {
                if let Some(start) = whitespace_start.take() {
                    normalized.push(' ');
                    offsets.push((start, position));
                }
                normalized.push(ch);
                offsets.push((position, position + length));
            }
            position += length;
        }
        (normalized, offsets)
    }

    let (needle, _) = collapsed(quote);
    if needle.trim().is_empty() {
        return None;
    }
    let (haystack, offsets) = collapsed(source);
    let position = haystack.find(&needle)?;
    let start_char = haystack[..position].chars().count();
    let end_char = start_char + needle.chars().count() - 1;
    Some(source[offsets[start_char].0..offsets[end_char].1].to_owned())
}

pub fn strip_outer_markdown_fence(input: &str) -> String {
    let input = input.trim();
    let Some(rest) = input.strip_prefix("```") else {
        return input.to_string();
    };
    let Some(content) = rest.strip_suffix("```") else {
        return input.to_string();
    };

    content.trim().to_string()
}

pub fn extract_known_paths(
    input: &str,
    known_paths: &[std::path::PathBuf],
    max_paths: usize,
) -> Vec<std::path::PathBuf> {
    let mut matches = Vec::new();

    for line in input.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if matches.len() >= max_paths {
            break;
        }

        let Some(path) = known_paths
            .iter()
            .filter(|path| line.contains(&path.to_string_lossy().replace('\\', "/")))
            .max_by_key(|path| path.as_os_str().len())
        else {
            continue;
        };

        if !matches.contains(path) {
            matches.push(path.clone());
        }
    }

    matches
}

pub fn extract_standalone_choice(input: &str, max_choice: usize) -> Option<usize> {
    input.lines().find_map(|line| {
        let mut line = line.trim();

        for marker in ["**", "__", "`"] {
            if line.starts_with(marker) && line.ends_with(marker) && line.len() >= marker.len() * 2
            {
                line = line[marker.len()..line.len() - marker.len()].trim();
            }
        }

        if line.len() >= 6 && line[..6].eq_ignore_ascii_case("design") {
            line = line[6..].trim();
        }

        line.parse::<usize>()
            .ok()
            .filter(|choice| (1..=max_choice).contains(choice))
    })
}

pub fn extract_standalone_keyword(input: &str, keywords: &[&str]) -> Option<String> {
    input.lines().find_map(|line| {
        let mut line = line.trim();

        for marker in ["**", "__"] {
            if line.starts_with(marker) && line.ends_with(marker) && line.len() >= marker.len() * 2
            {
                line = line[marker.len()..line.len() - marker.len()].trim();
            }
        }

        keywords
            .iter()
            .find(|keyword| line == **keyword)
            .map(|keyword| (*keyword).to_string())
    })
}

/// this is unfortunately necessary since some models does not
/// convert (escape) code correctly into json.
pub fn raw_fence_to_string(encoded: &str) -> String {
    const START_MARKER: &str = "RAW_TEXT_BEGIN>>\n";
    const CORE_END_MARKER: &str = "RAW_TEXT_END";

    let mut current_string = encoded.to_string();

    while let Some(found_start_pos) = current_string.find(START_MARKER) {
        // Optional '"' directly before RAW_TEXT_BEGIN>>\n
        let start_pos =
            if found_start_pos > 0 && current_string.as_bytes()[found_start_pos - 1] == b'"' {
                found_start_pos - 1
            } else {
                found_start_pos
            };

        let content_start = found_start_pos + START_MARKER.len();

        // Suche nach RAW_TEXT_END im restlichen Text
        let slice_after_start = &current_string[content_start..];
        let Some(core_end_offset) = slice_after_start.find(CORE_END_MARKER) else {
            break;
        };

        let core_end_pos = content_start + core_end_offset;

        // Falls der End-Marker als <<RAW_TEXT_END vorkommt,
        // gehört das << nicht zum eigentlichen Inhalt.
        let mut actual_end_pos = core_end_pos;

        while actual_end_pos > content_start {
            let prev_char = current_string.as_bytes()[actual_end_pos - 1];

            if prev_char == b'<' {
                actual_end_pos -= 1;
            } else {
                break;
            }
        }

        // Inhalt extrahieren
        let raw_content = &current_string[content_start..actual_end_pos];
        let conv = serde_json::to_string(raw_content).unwrap_or_default();

        // Ende des Markers bestimmen.
        // RAW_TEXT_END oder <<RAW_TEXT_END
        let mut tail_pos = core_end_pos + CORE_END_MARKER.len();

        while tail_pos < current_string.len() {
            let next_char = current_string.as_bytes()[tail_pos];

            // Bestehendes Verhalten beibehalten:
            // >, Newline, Carriage Return und Leerzeichen überspringen.
            if next_char == b'>' || next_char == b'\n' || next_char == b'\r' || next_char == b' ' {
                tail_pos += 1;
            } else {
                break;
            }
        }

        let tail = &current_string[tail_pos..];

        if tail.starts_with(',') || tail.starts_with('}') {
            current_string = format!("{}{}{}", &current_string[..start_pos], conv, tail);
        } else {
            current_string = format!("{}{},{}", &current_string[..start_pos], conv, tail);
        }
    }

    current_string
}
