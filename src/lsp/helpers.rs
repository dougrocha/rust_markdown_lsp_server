use std::str::FromStr;

use lsp_types::{Position, Range, Uri};
use miette::{Context, Result};
use ropey::RopeSlice;

use crate::{
    document::{
        references::{ReferenceKind, TargetHeader},
        Document,
    },
    get_document,
    lsp::server::Server,
    path::combine_and_normalize,
    Reference, TextBufferConversions,
};

/// Normalizes header content to match the format used in completions
pub fn normalize_header_content(content: &str) -> String {
    let mut result = content
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>();

    // Replace multiple consecutive dashes with single dash
    while result.contains("--") {
        result = result.replace("--", "-");
    }

    result.trim_matches('-').to_string()
}

/// Retrieves the content from a linked document based on the provided link data.
pub fn get_content(
    lsp: &Server,
    document: &Document,
    target: &str,
    header: Option<&TargetHeader>,
) -> Result<String> {
    let file_path = combine_and_normalize(&document.uri, &Uri::from_str(target).unwrap())?;

    let document = get_document!(&lsp, &file_path);

    let slice = document.content.slice(..);

    let Some(header_target) = header else {
        return Ok(slice.to_string());
    };

    let (extracted_content, _range) =
        extract_header_section(header_target, &document.references, slice);

    match extracted_content {
        Some(content) => Ok(content.to_string()),
        None => Ok(slice.to_string()),
    }
}

/// Extracts the content of a header section from the provided references.
///
/// A header section includes all content from the target header until:
/// - The next header of the same or higher level (lower number)
/// - The end of the document
///
/// Examples:
/// - H1 section continues until the next H1 or end of file
/// - H2 section continues until the next H1 or H2
/// - H3 section continues until the next H1, H2, or H3
pub fn extract_header_section<'a>(
    header: &TargetHeader,
    links: &[Reference],
    content: RopeSlice<'a>,
) -> (Option<RopeSlice<'a>>, Range) {
    let mut start_position: Option<Position> = None;
    let mut end_position: Option<Position> = None;

    let mut header_level: Option<usize> = None;

    for link in links {
        if let ReferenceKind::Header {
            level,
            content: header_content,
        } = &link.kind
        {
            // Check if this header matches our target header
            let target_content = header.content.strip_prefix('#').unwrap_or(&header.content);

            // Try multiple matching strategies:
            // 1. Exact match with stripped prefix
            // 2. Normalized target vs original content
            // 3. Normalized target vs normalized content
            let matches_header = *header_content == target_content
                || normalize_header_content(header_content) == target_content
                || normalize_header_content(header_content)
                    == normalize_header_content(target_content);

            if start_position.is_none() && matches_header {
                // Found the start of our target header section
                start_position = Some(link.range.start);
                header_level = Some(*level);
                continue;
            } else if start_position.is_some() && header_level.is_some_and(|h| *level <= h) {
                // Found a header of higher level (lower number) - this ends our section
                end_position = Some(link.range.start);
                break;
            }
        }
    }

    match (start_position, end_position) {
        (Some(start), Some(end)) if start < end && (end.line as usize) <= content.len_bytes() => {
            let start_idx = content.lsp_position_to_byte(start);
            let end_idx = content.lsp_position_to_byte(end);
            (
                Some(content.byte_slice(start_idx..end_idx)),
                Range::new(start, end),
            )
        }
        (Some(start), None) if (start.line as usize) < content.len_bytes() => {
            let start_idx = content.lsp_position_to_byte(start);
            (
                Some(content.byte_slice(start_idx..)),
                Range::new(start, Position::new(content.len_lines() as u32, 0)),
            )
        }
        _ => (None, Range::default()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_header_content() {
        assert_eq!(normalize_header_content("Example Header"), "example-header");
        assert_eq!(
            normalize_header_content("Example  Header"),
            "example-header"
        );
        assert_eq!(normalize_header_content("Example-Header"), "example-header");
        assert_eq!(normalize_header_content("Example_Header"), "example-header");
        assert_eq!(
            normalize_header_content("Example & Header"),
            "example-header"
        );
    }

    #[test]
    fn test_header_matching_with_hash_prefix() {
        let original_header = "Example Header";
        let target_with_hash = "#example-header";
        let target_without_hash = target_with_hash.strip_prefix('#').unwrap();

        // Test the matching logic used in goto_definition
        let matches = normalize_header_content(original_header) == target_without_hash
            || normalize_header_content(original_header)
                == normalize_header_content(target_without_hash);

        assert!(
            matches,
            "Header matching should work with hash prefix stripped"
        );
    }

    #[test]
    fn test_extract_header_section_hierarchy() {
        use crate::document::references::{Reference, ReferenceKind, TargetHeader};
        use lsp_types::{Position, Range};
        use ropey::Rope;

        // Create test content with nested headers
        let content = "# H1 Header\nContent under H1\n\n## H2 Header\nContent under H2\n\n### H3 Header\nContent under H3\n\n### Another H3\nMore H3 content\n\n## Another H2\nMore H2 content\n\n# Another H1\nMore H1 content";
        let rope = Rope::from_str(content);
        let slice = rope.slice(..);

        // Create references for headers
        let references = vec![
            Reference {
                kind: ReferenceKind::Header {
                    level: 1,
                    content: "H1 Header".to_string(),
                },
                range: Range::new(Position::new(0, 0), Position::new(0, 10)),
            },
            Reference {
                kind: ReferenceKind::Header {
                    level: 2,
                    content: "H2 Header".to_string(),
                },
                range: Range::new(Position::new(3, 0), Position::new(3, 12)),
            },
            Reference {
                kind: ReferenceKind::Header {
                    level: 3,
                    content: "H3 Header".to_string(),
                },
                range: Range::new(Position::new(6, 0), Position::new(6, 13)),
            },
            Reference {
                kind: ReferenceKind::Header {
                    level: 3,
                    content: "Another H3".to_string(),
                },
                range: Range::new(Position::new(9, 0), Position::new(9, 14)),
            },
            Reference {
                kind: ReferenceKind::Header {
                    level: 2,
                    content: "Another H2".to_string(),
                },
                range: Range::new(Position::new(12, 0), Position::new(12, 14)),
            },
            Reference {
                kind: ReferenceKind::Header {
                    level: 1,
                    content: "Another H1".to_string(),
                },
                range: Range::new(Position::new(15, 0), Position::new(15, 14)),
            },
        ];

        // Test H3 section extraction - should stop at next H3, H2, or H1
        let target_header = TargetHeader {
            level: 3,
            content: "H3 Header".to_string(),
        };
        let (extracted, _range) = extract_header_section(&target_header, &references, slice);

        assert!(extracted.is_some(), "Should extract H3 section");
        let extracted_text = extracted.unwrap().to_string();

        // Should include content under H3 but stop before "Another H3"
        assert!(
            extracted_text.contains("Content under H3"),
            "Should include H3 content"
        );
        assert!(
            !extracted_text.contains("Another H3"),
            "Should stop before next H3"
        );

        // Test H2 section extraction - should stop at next H2 or H1
        let target_header = TargetHeader {
            level: 2,
            content: "H2 Header".to_string(),
        };
        let (extracted, _range) = extract_header_section(&target_header, &references, slice);

        assert!(extracted.is_some(), "Should extract H2 section");
        let extracted_text = extracted.unwrap().to_string();

        // Should include H2 content and nested H3 sections but stop before "Another H2"
        assert!(
            extracted_text.contains("Content under H2"),
            "Should include H2 content"
        );
        assert!(
            extracted_text.contains("H3 Header"),
            "Should include nested H3"
        );
        assert!(
            extracted_text.contains("Another H3"),
            "Should include all H3s under this H2"
        );
        assert!(
            !extracted_text.contains("Another H2"),
            "Should stop before next H2"
        );
    }

    #[test]
    fn test_extract_header_section_edge_cases() {
        use crate::document::references::{Reference, ReferenceKind, TargetHeader};
        use lsp_types::{Position, Range};
        use ropey::Rope;

        // Test case: H1 section that goes to end of file
        let content = "# Main Header\nContent under main header\n\n## Sub Header\nSub content\n\nMore content at end";
        let rope = Rope::from_str(content);
        let slice = rope.slice(..);

        let references = vec![
            Reference {
                kind: ReferenceKind::Header {
                    level: 1,
                    content: "Main Header".to_string(),
                },
                range: Range::new(Position::new(0, 0), Position::new(0, 13)),
            },
            Reference {
                kind: ReferenceKind::Header {
                    level: 2,
                    content: "Sub Header".to_string(),
                },
                range: Range::new(Position::new(3, 0), Position::new(3, 12)),
            },
        ];

        // Test H1 extraction - should go to end of file
        let target_header = TargetHeader {
            level: 1,
            content: "Main Header".to_string(),
        };
        let (extracted, _range) = extract_header_section(&target_header, &references, slice);

        assert!(extracted.is_some(), "Should extract H1 section");
        let extracted_text = extracted.unwrap().to_string();

        // Should include everything from H1 to end of file
        assert!(
            extracted_text.contains("Content under main header"),
            "Should include H1 content"
        );
        assert!(
            extracted_text.contains("Sub Header"),
            "Should include nested H2"
        );
        assert!(
            extracted_text.contains("More content at end"),
            "Should include content to end of file"
        );

        // Test with hash prefix in target
        let target_header_with_hash = TargetHeader {
            level: 1,
            content: "#Main Header".to_string(),
        };
        let (extracted, _range) =
            extract_header_section(&target_header_with_hash, &references, slice);

        assert!(
            extracted.is_some(),
            "Should extract H1 section with hash prefix"
        );
    }
}
