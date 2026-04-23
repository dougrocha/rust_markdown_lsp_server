pub mod did_rename;
pub mod will_rename;

use std::collections::HashMap;

use lib_core::{document::references::ReferenceKind, get_document, uri::UriExt};
use lsp_types::{
    DocumentChangeOperation, DocumentChanges, OneOf, OptionalVersionedTextDocumentIdentifier,
    Position, PrepareRenameResponse, Range, RenameFile, RenameParams, ResourceOp, TextDocumentEdit,
    TextDocumentPositionParams, TextEdit, Uri, WorkspaceEdit,
};
use miette::{Context, Result, miette};

use crate::{
    handlers::link_resolver::resolve_target_uri,
    helpers::{generate_link_text, normalize_header_content},
    server::Server,
};

pub fn process_prepare_rename(
    lsp: &mut Server,
    params: TextDocumentPositionParams,
) -> Result<Option<PrepareRenameResponse>> {
    let uri = params.text_document.uri;
    let position = params.position;

    let document = get_document!(lsp, &uri);

    let response = match document.get_reference_at_position(position) {
        Some(r) => match &r.kind {
            ReferenceKind::Header { level, content } => {
                // Offer just the content text as the rename target, excluding the `## ` prefix
                let content_col = r.range.start.character + *level as u32 + 1;
                let content_range =
                    Range::new(Position::new(r.range.start.line, content_col), r.range.end);
                PrepareRenameResponse::RangeWithPlaceholder {
                    range: content_range,
                    placeholder: content.clone(),
                }
            }
            ReferenceKind::WikiLink { target, alias, .. } => {
                PrepareRenameResponse::RangeWithPlaceholder {
                    range: r.range,
                    placeholder: alias.clone().unwrap_or_else(|| target.clone()),
                }
            }
            ReferenceKind::Link { alt_text, .. } => PrepareRenameResponse::RangeWithPlaceholder {
                range: r.range,
                placeholder: alt_text.clone(),
            },
        },
        // Cursor not on a symbol — let the editor pick the word (renames current file)
        None => PrepareRenameResponse::DefaultBehavior {
            default_behavior: true,
        },
    };

    Ok(Some(response))
}

pub fn process_rename(lsp: &mut Server, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
    let uri = params.text_document_position.text_document.uri;
    let position = params.text_document_position.position;
    let new_name = params.new_name;

    // Extract reference data before dropping the borrow on lsp
    let reference_kind = {
        let document = get_document!(lsp, &uri);
        document
            .get_reference_at_position(position)
            .map(|r| r.kind.clone())
    };

    match reference_kind {
        Some(ReferenceKind::Header { level, content }) => {
            rename_header(&*lsp, &uri, level, &content, &new_name)
        }

        Some(ReferenceKind::Link { target, .. } | ReferenceKind::WikiLink { target, .. }) => {
            // Resolve the link target, then rename that file
            let target_uri = {
                let doc = get_document!(lsp, &uri);
                resolve_target_uri(&*lsp, doc, &target)
                    .with_context(|| format!("Could not resolve link target '{}'", target))?
            };
            rename_file(&*lsp, &target_uri, &new_name)
        }

        // Cursor not on any reference — rename the current file
        None => rename_file(&*lsp, &uri, &new_name),
    }
}

fn rename_header(
    lsp: &Server,
    current_uri: &Uri,
    level: usize,
    old_content: &str,
    new_name: &str,
) -> Result<Option<WorkspaceEdit>> {
    let new_normalized = normalize_header_content(new_name);
    let mut changes: HashMap<Uri, Vec<TextEdit>> = HashMap::new();

    // Collect a snapshot of all references to avoid borrow-while-iterating issues
    let all_refs: Vec<(Uri, ReferenceKind, Range)> = lsp
        .documents
        .get_references_with_uri()
        .map(|(uri, r)| (uri.clone(), r.kind.clone(), r.range))
        .collect();

    // 1. Rewrite the header line in the current document
    for (ref_uri, ref_kind, ref_range) in &all_refs {
        if ref_uri != current_uri {
            continue;
        }
        if let ReferenceKind::Header { level: l, content } = ref_kind
            && *l == level
            && content == old_content
        {
            let new_header_text = format!("{} {}", "#".repeat(level), new_name);
            changes.entry(ref_uri.clone()).or_default().push(TextEdit {
                range: *ref_range,
                new_text: new_header_text,
            });
            break;
        }
    }

    // 2. Rewrite every cross-file link that points to this header
    for (ref_uri, ref_kind, ref_range) in &all_refs {
        let (target, header_opt) = match ref_kind {
            ReferenceKind::WikiLink { target, header, .. } => (target.as_str(), header.as_deref()),
            ReferenceKind::Link { target, header, .. } => (target.as_str(), header.as_deref()),
            ReferenceKind::Header { .. } => continue,
        };

        // Only care about links that actually name a header
        let Some(link_header) = header_opt else {
            continue;
        };

        // Check header slug matches
        if !header_slugs_match(link_header, old_content) {
            continue;
        }

        // Check the link resolves to the file we are renaming inside
        let containing_doc = match lsp.documents.get_document(ref_uri) {
            Some(d) => d,
            None => continue,
        };

        let resolved = match resolve_target_uri(lsp, containing_doc, target) {
            Ok(u) => u,
            Err(_) => continue,
        };

        if &resolved != current_uri {
            continue;
        }

        // Reconstruct the full link with the updated header slug
        let new_link = reconstruct_link_with_header(ref_kind, &new_normalized);
        changes.entry(ref_uri.clone()).or_default().push(TextEdit {
            range: *ref_range,
            new_text: new_link,
        });
    }

    if changes.is_empty() {
        return Ok(None);
    }

    Ok(Some(WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    }))
}

fn rename_file(lsp: &Server, old_uri: &Uri, new_name: &str) -> Result<Option<WorkspaceEdit>> {
    // Build the new URI by swapping the filename stem
    let old_path = old_uri
        .to_file_path()
        .ok_or_else(|| miette!("Cannot convert URI to file path: {:?}", old_uri))?;

    let new_stem = new_name.strip_suffix(".md").unwrap_or(new_name);
    let new_path = old_path.with_file_name(format!("{}.md", new_stem));

    let new_uri = Uri::from_file_path(&new_path)
        .ok_or_else(|| miette!("Cannot create URI from path: {:?}", new_path))?;

    // Snapshot all references for borrow safety
    let all_refs: Vec<(Uri, ReferenceKind, Range)> = lsp
        .documents
        .get_references_with_uri()
        .map(|(uri, r)| (uri.clone(), r.kind.clone(), r.range))
        .collect();

    // Gather text edits for every inbound link
    let mut text_edits: HashMap<Uri, Vec<TextEdit>> = HashMap::new();

    for (ref_uri, ref_kind, ref_range) in &all_refs {
        let target = match ref_kind.get_target() {
            Some(t) => t.to_string(),
            None => continue,
        };

        let containing_doc = match lsp.documents.get_document(ref_uri) {
            Some(d) => d,
            None => continue,
        };

        let resolved = match resolve_target_uri(lsp, containing_doc, &target) {
            Ok(u) => u,
            Err(_) => continue,
        };

        if &resolved != old_uri {
            continue;
        }

        // Generate a new link target string honouring the configured link style
        let workspace_root = lsp.get_workspace_root_for_uri(ref_uri);
        let new_target = generate_link_text(&lsp.config.links, ref_uri, &new_uri, workspace_root)
            .unwrap_or_else(|_| new_stem.to_string());

        let new_link = reconstruct_link_with_target(ref_kind, &new_target);
        text_edits
            .entry(ref_uri.clone())
            .or_default()
            .push(TextEdit {
                range: *ref_range,
                new_text: new_link,
            });
    }

    // Build workspace edit: file rename resource op + text edits in one transaction
    let mut operations: Vec<DocumentChangeOperation> = vec![DocumentChangeOperation::Op(
        ResourceOp::Rename(RenameFile {
            old_uri: old_uri.clone(),
            new_uri,
            options: None,
            annotation_id: None,
        }),
    )];

    for (doc_uri, edits) in text_edits {
        operations.push(DocumentChangeOperation::Edit(TextDocumentEdit {
            text_document: OptionalVersionedTextDocumentIdentifier {
                uri: doc_uri,
                version: None,
            },
            edits: edits.into_iter().map(OneOf::Left).collect(),
        }));
    }

    Ok(Some(WorkspaceEdit {
        document_changes: Some(DocumentChanges::Operations(operations)),
        ..Default::default()
    }))
}

/// Returns true when a link's `#header` fragment matches a header's content,
/// accounting for different normalisation styles.
fn header_slugs_match(link_header: &str, header_content: &str) -> bool {
    let stripped = link_header.strip_prefix('#').unwrap_or(link_header);
    normalize_header_content(stripped) == normalize_header_content(header_content)
}

/// Rebuild a WikiLink or InlineLink with an updated `#header` slug.
fn reconstruct_link_with_header(kind: &ReferenceKind, new_header_slug: &str) -> String {
    match kind {
        ReferenceKind::WikiLink { target, alias, .. } => match alias {
            Some(a) => format!("[[{}#{}|{}]]", target, new_header_slug, a),
            None => format!("[[{}#{}]]", target, new_header_slug),
        },
        ReferenceKind::Link {
            target, alt_text, ..
        } => format!("[{}]({}#{})", alt_text, target, new_header_slug),
        ReferenceKind::Header { .. } => unreachable!("headers are filtered before this call"),
    }
}

/// Rebuild a WikiLink or InlineLink with an updated file target.
fn reconstruct_link_with_target(kind: &ReferenceKind, new_target: &str) -> String {
    match kind {
        ReferenceKind::WikiLink { alias, header, .. } => {
            let target_part = match header {
                Some(h) => format!("{}#{}", new_target, h),
                None => new_target.to_string(),
            };
            match alias {
                Some(a) => format!("[[{}|{}]]", target_part, a),
                None => format!("[[{}]]", target_part),
            }
        }
        ReferenceKind::Link {
            alt_text, header, ..
        } => {
            let target_part = match header {
                Some(h) => format!("{}#{}", new_target, h),
                None => new_target.to_string(),
            };
            format!("[{}]({})", alt_text, target_part)
        }
        ReferenceKind::Header { .. } => unreachable!("headers are filtered before this call"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_slugs_match() {
        assert!(header_slugs_match("my-header", "My Header"));
        assert!(header_slugs_match("#my-header", "My Header"));
        assert!(header_slugs_match("my-header", "my-header"));
        assert!(!header_slugs_match("other-header", "My Header"));
        assert!(!header_slugs_match("#other", "My Header"));
    }

    #[test]
    fn test_reconstruct_wikilink_with_header() {
        let kind = ReferenceKind::WikiLink {
            target: "my-file".to_string(),
            alias: None,
            header: Some("old-header".to_string()),
        };
        assert_eq!(
            reconstruct_link_with_header(&kind, "new-header"),
            "[[my-file#new-header]]"
        );
    }

    #[test]
    fn test_reconstruct_wikilink_with_header_and_alias() {
        let kind = ReferenceKind::WikiLink {
            target: "my-file".to_string(),
            alias: Some("My Alias".to_string()),
            header: Some("old-header".to_string()),
        };
        assert_eq!(
            reconstruct_link_with_header(&kind, "new-header"),
            "[[my-file#new-header|My Alias]]"
        );
    }

    #[test]
    fn test_reconstruct_inline_link_with_header() {
        let kind = ReferenceKind::Link {
            target: "my-file.md".to_string(),
            alt_text: "click here".to_string(),
            title: None,
            header: Some("old".to_string()),
        };
        assert_eq!(
            reconstruct_link_with_header(&kind, "new"),
            "[click here](my-file.md#new)"
        );
    }

    #[test]
    fn test_reconstruct_wikilink_with_target() {
        let kind = ReferenceKind::WikiLink {
            target: "old-file".to_string(),
            alias: None,
            header: None,
        };
        assert_eq!(
            reconstruct_link_with_target(&kind, "new-file"),
            "[[new-file]]"
        );
    }

    #[test]
    fn test_reconstruct_wikilink_with_target_keeps_header() {
        let kind = ReferenceKind::WikiLink {
            target: "old-file".to_string(),
            alias: Some("label".to_string()),
            header: Some("section".to_string()),
        };
        assert_eq!(
            reconstruct_link_with_target(&kind, "new-file"),
            "[[new-file#section|label]]"
        );
    }

    #[test]
    fn test_reconstruct_inline_link_with_target() {
        let kind = ReferenceKind::Link {
            target: "old.md".to_string(),
            alt_text: "text".to_string(),
            title: None,
            header: None,
        };
        assert_eq!(
            reconstruct_link_with_target(&kind, "new.md"),
            "[text](new.md)"
        );
    }

    #[test]
    fn test_reconstruct_inline_link_with_target_keeps_header() {
        let kind = ReferenceKind::Link {
            target: "old.md".to_string(),
            alt_text: "text".to_string(),
            title: None,
            header: Some("intro".to_string()),
        };
        assert_eq!(
            reconstruct_link_with_target(&kind, "new.md"),
            "[text](new.md#intro)"
        );
    }
}
