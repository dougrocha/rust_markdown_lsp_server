use lsp_types::Uri;
use miette::{miette, Result};
use std::str::FromStr;

use crate::{
    config::LinkConfig,
    document::Document,
    lsp::server::DocumentStore,
    path::combine_and_normalize,
    UriExt,
};

/// Main entry point for link resolution
///
/// Resolution order:
/// 1. If target looks like a path (contains /), resolve as path
/// 2. If filename resolution enabled, try to find by filename
/// 3. Fallback to treating as relative path
pub fn resolve_link(
    target: &str,
    source_doc: &Document,
    config: &LinkConfig,
    documents: &DocumentStore,
    workspace_root: Option<&Uri>,
) -> Result<Uri> {
    // Step 1: Check if it's path syntax
    if is_path_syntax(target) {
        return resolve_as_path(target, source_doc, workspace_root);
    }

    // Step 2: Try filename resolution (if enabled)
    if config.enable_filename_resolution {
        if let Some(resolved) = resolve_by_filename(target, documents, config) {
            return Ok(resolved);
        }
    }

    // Step 3: Fallback to relative path
    resolve_as_path(target, source_doc, workspace_root)
}

/// Check if target uses path syntax (has slashes or path prefixes)
fn is_path_syntax(target: &str) -> bool {
    target.starts_with('/') ||      // Absolute: /docs/note.md
    target.starts_with("./") ||     // Relative: ./note.md
    target.starts_with("../") ||    // Relative parent: ../note.md
    target.contains('/')            // Has path separator: folder/note.md
}

/// Resolve target as a file path (relative or absolute)
fn resolve_as_path(
    target: &str,
    source_doc: &Document,
    workspace_root: Option<&Uri>,
) -> Result<Uri> {
    if target.starts_with('/') {
        // Absolute path - resolve relative to workspace root
        let Some(root) = workspace_root else {
            return Err(miette!("No workspace root available for absolute path: {}", target));
        };
        
        let root_path = root
            .to_file_path()
            .ok_or_else(|| miette!("Failed to convert workspace root to file path"))?;
        
        let target_path = root_path.join(target.strip_prefix('/').unwrap_or(target));
        
        Uri::from_file_path(target_path)
            .ok_or_else(|| miette!("Failed to create URI from absolute path: {}", target))
    } else {
        // Relative path - resolve relative to source document
        let target_uri = Uri::from_str(target)
            .map_err(|e| miette!("Invalid URI: {} - {}", target, e))?;
        
        combine_and_normalize(&source_doc.uri, &target_uri)
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
    // Always strip .md extension from target for comparison
    let target_stem = target.strip_suffix(".md").unwrap_or(target);
    
    let normalized_target = normalize_for_matching(target_stem);
    
    // Search all documents for matching filename
    for doc in documents.get_documents() {
        if let Some(doc_filename) = extract_filename_stem(&doc.uri) {
            let normalized_doc = normalize_for_matching(&doc_filename);
            
            if normalized_target == normalized_doc {
                return Some(doc.uri.clone());
            }
        }
    }
    
    None
}

/// Extract filename without extension from URI
/// Example: file:///path/to/note.md -> Some("note")
pub fn extract_filename_stem(uri: &Uri) -> Option<String> {
    let path = uri.to_file_path()?;
    let filename = path.file_stem()?;
    Some(filename.to_string_lossy().into_owned())
}

/// Extract full filename with extension from URI
/// Example: file:///path/to/note.md -> Some("note.md")
pub fn extract_filename(uri: &Uri) -> Option<String> {
    let path = uri.to_file_path()?;
    let filename = path.file_name()?;
    Some(filename.to_string_lossy().into_owned())
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
            ' ' | '_' => '-',
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

    #[test]
    fn test_extract_filename_stem() {
        let uri = Uri::from_str("file:///path/to/note.md").unwrap();
        assert_eq!(extract_filename_stem(&uri), Some("note".to_string()));

        let uri = Uri::from_str("file:///my-note.md").unwrap();
        assert_eq!(extract_filename_stem(&uri), Some("my-note".to_string()));
    }
}
