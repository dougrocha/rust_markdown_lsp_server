pub mod did_rename;
pub mod will_rename;

use std::ops::Range;
use std::path::Path;

use gen_lsp_types::{PrepareRenameParams, PrepareRenameResult, RenameParams, WorkspaceEdit};
use lib_core::{
    document::Document,
    path::find_relative_path,
    resolver::{self, TargetResolution, VaultContext},
};
use miette::Result;

use crate::server_state::ServerState;

/// Return `(span, replacement)` for each link in a moved file that must change so it still resolves from `new_path`.
pub(crate) fn moved_file_link_edits(
    doc: &Document,
    old_path: &Path,
    new_path: &Path,
    cx: &VaultContext<'_>,
) -> Vec<(Range<usize>, String)> {
    doc.links()
        .filter_map(|link| {
            let target = link.target_str(&doc.source);
            if resolver::is_external(&target) {
                return None;
            }

            let before = match resolver::resolve(&target, old_path, cx) {
                TargetResolution::File(path) => path,
                TargetResolution::Unresolved => return None,
            };

            // Skip a link that still resolves to the same file from the new path.
            if let TargetResolution::File(after) = resolver::resolve(&target, new_path, cx)
                && after == before
            {
                return None;
            }

            let new_rel = find_relative_path(new_path, &before).ok()?;
            Some((link.span.into(), link.render_with_target(&doc.source, &new_rel)))
        })
        .collect()
}

/// Apply each `(span, replacement)` edit to `text` from the last span first, so earlier offsets stay valid.
pub(crate) fn apply_edits(text: &mut String, mut edits: Vec<(Range<usize>, String)>) {
    edits.sort_by_key(|edit| std::cmp::Reverse(edit.0.start));
    for (span, replacement) in edits {
        text.replace_range(span, &replacement);
    }
}

pub fn process_prepare_rename(
    _lsp: &mut ServerState,
    params: PrepareRenameParams,
) -> Result<Option<PrepareRenameResult>> {
    let _uri = params.text_document_position_params.text_document.uri;
    let _position = params.text_document_position_params.position;

    // TODO: This was written by AI only, its garbage, Ima redo it eventually using
    // will_rename as the example I wrote

    // let response = match document.get_reference_at_position(position) {
    //     Some(r) => match &r.kind {
    //         ReferenceKind::Header { level, content } => {
    //             let content_col = r.range.start.character + *level as u32 + 1;
    //             let content_range =
    //                 Range::new(Position::new(r.range.start.line, content_col), r.range.end);
    //             PrepareRenameResult::PrepareRenamePlaceholder(PrepareRenamePlaceholder {
    //                 range: content_range,
    //                 placeholder: content.clone(),
    //             })
    //         }
    //         ReferenceKind::WikiLink { target, alias, .. } => {
    //             PrepareRenameResult::PrepareRenamePlaceholder(PrepareRenamePlaceholder {
    //                 range: r.range,
    //                 placeholder: alias.clone().unwrap_or_else(|| target.clone()),
    //             })
    //         }
    //         ReferenceKind::Link { alt_text, .. } => {
    //             PrepareRenameResult::PrepareRenamePlaceholder(PrepareRenamePlaceholder {
    //                 range: r.range,
    //                 placeholder: alt_text.clone(),
    //             })
    //         },
    //     },
    //     // Cursor not on a symbol — let the editor pick the word (renames current file)
    //     None => PrepareRenameResult::PrepareRenameDefaultBehavior(PrepareRenameDefaultBehavior {
    //         default_behavior: true,
    //     }),
    // };

    Ok(None)
}

pub fn process_rename(
    _lsp: &mut ServerState,
    params: RenameParams,
) -> Result<Option<WorkspaceEdit>> {
    let _uri = params.text_document_position_params.text_document.uri;
    let _position = params.text_document_position_params.position;
    let _new_name = params.new_name;

    // // Extract reference data before dropping the borrow on lsp
    // let reference_kind = {
    //     let document = get_document!(lsp, &uri);
    //     document
    //         .get_reference_at_position(position)
    //         .map(|r| r.kind.clone())
    // };
    //
    // match reference_kind {
    //     Some(ReferenceKind::Header { level, content }) => {
    //         rename_header(&*lsp, &uri, level, &content, &new_name)
    //     }
    //
    //     Some(ReferenceKind::Link { target, .. } | ReferenceKind::WikiLink { target, .. }) => {
    //         // Resolve the link target, then rename that file
    //         let target_uri = {
    //             let doc = get_document!(lsp, &uri);
    //             resolve_target_uri(&*lsp, doc, &target)
    //                 .with_context(|| format!("Could not resolve link target '{}'", target))?
    //         };
    //         rename_file(&*lsp, &target_uri, &new_name)
    //     }
    //
    //     // Cursor not on any reference — rename the current file
    //     None => rename_file(&*lsp, &uri, &new_name),
    // }

    Ok(None)
}

#[cfg(test)]
mod tests {}
