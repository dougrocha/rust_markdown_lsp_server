use gen_lsp_types::Uri;
use miette::{Result, miette};

use lib_core::{
    config::LinkConfig,
    document::Document,
    path::{combine_and_normalize, extract_filename_stem, slug::filename_slug},
    vault::Vault,
};

use crate::{ServerState, uri::UriExt};

// TODO: Rethinl this whole flow here
// I need to try to make this non lsp specific so remove server state and just make it
// file name/str/path dependant
//
// In this case it should only use the Documenets and/or the new Vault system.
//
// After everthing, move this all to lsp-core

pub fn resolve_target_uri(lsp: &ServerState, document: &Document, target: &str) -> Result<Uri> {
    let active_root = lsp.get_workspace_root_for_path(&document.path);

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
    documents: &Vault,
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
        let resolved =
            combine_and_normalize(&source_doc.path, target).map_err(|e| miette!("{e}"))?;

        Uri::from_file_path(resolved)
            .ok_or_else(|| miette!("Failed to create URI from resolved path: {}", target))
    }
}

/// Resolve target by searching for matching filename in workspace
/// Returns first match found (optimized for speed)
///
/// Note: Always strips .md extension from target for comparison,
/// so both [[note]] and [[note.md]] will match note.md
fn resolve_by_filename(target: &str, documents: &Vault, _config: &LinkConfig) -> Option<Uri> {
    let target_stem = target.strip_suffix(".md").unwrap_or(target);

    let normalized_target = filename_slug(target_stem);
    documents.iter().find_map(|doc| {
        let doc_filename = extract_filename_stem(&doc.path)?;
        let normalized_doc = filename_slug(&doc_filename);

        if normalized_target == normalized_doc {
            Uri::from_file_path(&doc.path)
        } else {
            None
        }
    })
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
}
