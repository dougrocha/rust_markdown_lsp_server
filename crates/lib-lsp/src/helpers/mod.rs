pub mod references;

use gen_lsp_types::{Position, Range, Uri};
use miette::{Context, Result, miette};
use ropey::RopeSlice;

use lib_core::{
    config::{LinkConfig, LinkGenerationStyle},
    document::{
        Document,
        references::{Reference, ReferenceKind},
    },
    path::{extract_filename_stem, find_relative_path, slug::header_slug},
    text_buffer_conversions::TextBufferConversions,
};

use crate::{
    get_document, handlers::link_resolver::resolve_target_uri, server_state::ServerState,
    uri::UriExt,
};

/// Retrieves the content from a linked document based on the provided link data.
pub fn get_content(
    lsp: &ServerState,
    document: &Document,
    target: &str,
    header: Option<&str>,
) -> Result<String> {
    let target_uri = resolve_target_uri(lsp, document, target)?;

    let document = get_document!(&lsp, &target_uri);

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

/// Generate link text for a target document based on configuration
pub fn generate_link_text(
    config: &LinkConfig,
    source_uri: &Uri,
    target_uri: &Uri,
    workspace_root: Option<&Uri>,
) -> Result<String> {
    match config.generation_style {
        // Always use stem (no .md extension) for filename-based links
        LinkGenerationStyle::Filename => {
            let target_path = target_uri
                .to_file_path()
                .ok_or_else(|| miette!("Failed to convert target URI to path: {:?}", target_uri))?;
            Ok(extract_filename_stem(&target_path)
                .ok_or_else(|| miette!("Failed to extract filename stem from {:?}", target_uri))?)
        }
        LinkGenerationStyle::Relative => Ok(find_relative_path(
            source_uri.to_string(),
            target_uri.to_string(),
        )?),
        LinkGenerationStyle::Absolute => {
            if let Some(root) = workspace_root {
                generate_absolute_path(root, target_uri)
            } else {
                // Fallback to relative if no workspace root
                Ok(find_relative_path(
                    source_uri.to_string(),
                    target_uri.to_string(),
                )?)
            }
        }
    }
}

/// Generate absolute path from workspace root
fn generate_absolute_path(root: &Uri, target: &Uri) -> Result<String> {
    let root_path = root
        .to_file_path()
        .ok_or_else(|| miette!("Failed to convert root URI to path: {:?}", root))?;

    let target_path = target
        .to_file_path()
        .ok_or_else(|| miette!("Failed to convert target URI to path: {:?}", target))?;

    let relative = target_path.strip_prefix(&root_path).map_err(|_| {
        miette!(
            "Target URI {:?} is not within workspace root {:?}",
            target,
            root
        )
    })?;

    // Normalize this to forward slashes
    let components: Vec<&str> = relative
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(os_str) => os_str.to_str(),
            _ => None,
        })
        .collect();

    Ok(format!("/{}", components.join("/")))
}

/// Extracts the content of a header section from the provided references.
/// NOTE: Assumes `links` are sorted by position (top to bottom).
pub fn extract_header_section<'a>(
    header: &str,
    links: &[Reference],
    content: RopeSlice<'a>,
) -> (Option<RopeSlice<'a>>, Range) {
    let mut start_position: Option<Position> = None;
    let mut end_position: Option<Position> = None;
    let mut header_level: Option<usize> = None;

    // Optimization: Pre-calculate normalized target once
    let target_content = header.strip_prefix('#').unwrap_or(header);
    let normalized_target = header_slug(target_content);

    for link in links {
        if let ReferenceKind::Header {
            level,
            content: header_content,
        } = &link.kind
        {
            // Logic: Find start
            if start_position.is_none() {
                let matches_header = *header_content == target_content
                    || header_slug(header_content) == normalized_target;

                if matches_header {
                    start_position = Some(link.range.start);
                    header_level = Some(*level);
                }
                continue;
            }

            // Logic: Find end (must be after start, which loop order guarantees if sorted)
            // Stop at any header that is same level or higher (smaller number)
            if let Some(current_level) = header_level
                && *level <= current_level
            {
                end_position = Some(link.range.start);
                break;
            }
        }
    }

    match (start_position, end_position) {
        (Some(start), Some(end)) if start < end && (end.line as usize) <= content.len_lines() => {
            // Safety check: ensure positions are valid for this content
            if let (Some(start_byte), Some(end_byte)) = (
                content.try_position_to_byte_offset(start),
                content.try_position_to_byte_offset(end),
            ) {
                (
                    Some(content.byte_slice(start_byte..end_byte)),
                    Range::new(start, end),
                )
            } else {
                (None, Range::default())
            }
        }
        (Some(start), None) if (start.line as usize) < content.len_lines() => {
            if let Some(start_byte) = content.try_position_to_byte_offset(start) {
                (
                    Some(content.byte_slice(start_byte..)),
                    Range::new(start, Position::new(content.len_lines() as u32, 0)),
                )
            } else {
                (None, Range::default())
            }
        }
        _ => (None, Range::default()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_header_section_hierarchy() {
        // Create test content with nested headers
        let input = "# H1 Header\nContent under H1\n\n## H2 Header\nContent under H2\n\n### H3 Header\nContent under H3\n\n### Another H3\nMore H3 content\n\n## Another H2\nMore H2 content\n\n# Another H1\nMore H1 content";

        let document = Document::new(std::path::PathBuf::from("/TEST.md"), input, 0).unwrap();
        let references = document.references;
        let content = document.content.slice(..);

        // Test H3 section extraction - should stop at next H3, H2, or H1
        let target_header = "H3 Header".to_string();
        let (extracted, _range) = extract_header_section(&target_header, &references, content);

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
        let target_header = "H2 Header".to_string();
        let (extracted, _range) = extract_header_section(&target_header, &references, content);

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
    fn test_resolve_target_uri() {
        use crate::server_state::ServerState;
        use gen_lsp_types::Uri;
        use std::str::FromStr;

        let mut server = ServerState::new();
        let workspace_root = Uri::from_str("file:///workspace").unwrap();
        server.insert_root(workspace_root);

        let document = Document::new(
            std::path::PathBuf::from("/workspace/docs/test.md"),
            "# Test",
            1,
        )
        .unwrap();

        // Test absolute path resolution
        let result = resolve_target_uri(&server, &document, "/AGENTS.md");
        assert!(result.is_ok(), "Should resolve absolute path");
        let resolved_uri = result.unwrap();
        assert_eq!(&resolved_uri.to_string(), "file:///workspace/AGENTS.md");
    }

    #[test]
    fn test_extract_header_section_edge_cases() {
        use gen_lsp_types::{Position, Range};
        use lib_core::document::references::{Reference, ReferenceKind};
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
        let target_header = "Main Header".to_string();
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
        let target_header_with_hash = "#Main Header";
        let (extracted, _range) =
            extract_header_section(target_header_with_hash, &references, slice);

        assert!(
            extracted.is_some(),
            "Should extract H1 section with hash prefix"
        );
    }
}
