use lsp_types::Uri;
use miette::{Context, Result, miette};

use lib_core::{
    document::Document,
    path::{combine_and_normalize, extract_filename_stem},
    uri::UriExt,
};

use crate::{Server, config::LinkConfig, server::DocumentStore};

pub fn resolve_target_uri(lsp: &Server, document: &Document, target: &str) -> Result<Uri> {
    let active_root = lsp.get_workspace_root_for_uri(&document.uri);

    resolve_link(
        target,
        document,
        &lsp.config.links,
        &lsp.documents,
        active_root,
    )
}

/// Main entry point for link resolution
pub fn resolve_link(
    target: &str,
    source_doc: &Document,
    config: &LinkConfig,
    documents: &DocumentStore,
    workspace_root: Option<&Uri>,
) -> Result<Uri> {
    if is_path_syntax(target) {
        return resolve_as_path(target, source_doc, workspace_root);
    }

    if config.enable_filename_resolution
        && let Some(resolved) = resolve_by_filename(target, documents, config)
    {
        return Ok(resolved);
    }

    resolve_as_path(target, source_doc, workspace_root)
}

/// Check if target uses path syntax
fn is_path_syntax(target: &str) -> bool {
    target.starts_with('/') ||      // Absolute: /docs/note.md
    target.starts_with("./") ||     // Relative: ./note.md
    target.starts_with("../") ||    // Relative parent: ../note.md
    target.contains('/') || // Has path separator: folder/note.md
    target.contains('\\') // Windows Specific
}

fn resolve_as_path(
    target: &str,
    source_doc: &Document,
    workspace_root: Option<&Uri>,
) -> Result<Uri> {
    if target.starts_with('/') {
        // Absolute path - resolve relative to workspace root
        let Some(root) = workspace_root else {
            return Err(miette!(
                "No workspace root available for absolute path: {}",
                target
            ));
        };

        let root_path = root
            .to_file_path()
            .ok_or_else(|| miette!("Failed to convert workspace root to file path"))?;

        let target_path = root_path.join(target.strip_prefix('/').unwrap_or(target));

        Uri::from_file_path(target_path)
            .ok_or_else(|| miette!("Failed to create URI from absolute path: {}", target))
    } else {
        combine_and_normalize(&source_doc.uri, target).context(format!(
            "Failed to resolve relative path '{}' from '{}'",
            target,
            source_doc.uri.as_str()
        ))
    }
}

/// Resolve target by searching for matching filename in workspace
/// Returns first match found (optimized for speed)
///
/// Note: Always strips .md extension from target for comparison,
/// so both [[note]] and [[note.md]] will match note.md
fn resolve_by_filename(
    target: &str,
    documents: &DocumentStore,
    _config: &LinkConfig,
) -> Option<Uri> {
    let target_stem = target.strip_suffix(".md").unwrap_or(target);

    let normalized_target = normalize_for_matching(target_stem);
    documents.iter().find_map(|doc| {
        let doc_filename = extract_filename_stem(&doc.uri)?;
        let normalized_doc = normalize_for_matching(&doc_filename);

        if normalized_target == normalized_doc {
            Some(doc.uri.clone())
        } else {
            None
        }
    })
}

/// Normalize string for matching (case-insensitive, unified separators)
///
/// Transformations:
/// - Lowercase
/// - Spaces, dashes, underscores -> all become dashes
///
/// Examples:
/// - "My Note" -> "my-note"
/// - "my_note" -> "my-note"
/// - "MY-NOTE" -> "my-note"
fn normalize_for_matching(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| match c {
            ' ' | '_' | '-' => '-',
            c => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_path_syntax() {
        assert!(is_path_syntax("/absolute/path.md"));
        assert!(is_path_syntax("./relative.md"));
        assert!(is_path_syntax("../parent.md"));
        assert!(is_path_syntax("folder/file.md"));
        assert!(!is_path_syntax("note"));
        assert!(!is_path_syntax("my-note"));
    }

    #[test]
    fn test_normalize_for_matching() {
        assert_eq!(normalize_for_matching("My Note"), "my-note");
        assert_eq!(normalize_for_matching("my_note"), "my-note");
        assert_eq!(normalize_for_matching("MY-NOTE"), "my-note");
        assert_eq!(normalize_for_matching("my note"), "my-note");
        assert_eq!(normalize_for_matching("My_Cool_Note"), "my-cool-note");
    }
}
