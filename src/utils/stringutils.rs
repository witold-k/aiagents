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

    // 3. Otherwise raw text
    input.trim().to_string()
}

pub fn raw_fence_to_string(encoded: &str) -> String {
    const START_MARKER: &str = "RAW_TEXT_BEGIN>>\n";
    const CORE_END_MARKER: &str = "RAW_TEXT_END";

    // Wir arbeiten auf einer veränderbaren Kopie des Strings
    let mut current_string = encoded.to_string();

    // Schleife läuft so lange, wie noch unkonvertierte START_MARKER existieren
    while let Some(start_pos) = current_string.find(START_MARKER) {
        let content_start = start_pos + START_MARKER.len();

        // Suche nach dem Kernwort des End-Markers im restlichen Text
        let slice_after_start = &current_string[content_start..];
        let Some(core_end_offset) = slice_after_start.find(CORE_END_MARKER) else {
            // Falls kein End-Marker existiert, abbrechen um Endlosschleife zu verhindern
            break;
        };

        // Absolute Position des Kernworts im Gesamtstring
        let core_end_pos = content_start + core_end_offset;

        // Rückwärts gehen, um optionale Zeichen wie '<' oder '/' vor dem Marker zu finden
        let mut actual_end_pos = core_end_pos;
        while actual_end_pos > content_start {
            let prev_char = current_string.as_bytes()[actual_end_pos - 1];
            if prev_char == b'<' || prev_char == b'/' || prev_char == b' ' {
                actual_end_pos -= 1;
            } else {
                break;
            }
        }

        // Inhalt extrahieren (endet genau vor den optionalen Klammern/Slashes des End-Markers)
        let raw_content = &current_string[content_start..actual_end_pos];
        let conv = serde_json::to_string(raw_content).unwrap_or_default();

        // Vorwärts gehen, um das Ende des gesamten End-Markers inklusive anhängendem Müll zu finden
        let mut tail_pos = core_end_pos + CORE_END_MARKER.len();
        while tail_pos < current_string.len() {
            let next_char = current_string.as_bytes()[tail_pos];
            // Überspringe verbleibende Klammern, Newlines oder Leerzeichen direkt nach dem Marker
            if next_char == b'>' || next_char == b'\n' || next_char == b'\r' || next_char == b' ' {
                tail_pos += 1;
            } else {
                break;
            }
        }

        let tail = &current_string[tail_pos..];

        // Neuen Teilstring für den aktuellen Schleifendurchlauf zusammenbauen
        if tail.starts_with(',') {
            current_string = format!("{}{}{}", &current_string[..start_pos], conv, tail);
        } else {
            if tail.starts_with('}') {
                current_string = format!("{}{}{}", &current_string[..start_pos], conv, tail);
            } else {
                current_string = format!("{}{},{}", &current_string[..start_pos], conv, tail);
            }
        }
    }

    current_string
}

