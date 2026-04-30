pub mod did_rename;
pub mod will_rename;

use gen_lsp_types::{PrepareRenameParams, PrepareRenameResult, RenameParams, WorkspaceEdit};
use miette::Result;

use crate::server_state::ServerState;

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
