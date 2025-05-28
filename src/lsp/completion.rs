use lsp_types::{CompletionItem, CompletionItemKind, CompletionParams, CompletionResponse};
use miette::{Context, Result};

use super::server::Server;

pub fn process_completion(
    lsp: &mut Server,
    params: CompletionParams,
) -> Result<Option<CompletionResponse>> {
    log::debug!("Completion params: {:?}", params);

    let uri = params.text_document_position.text_document.uri;
    let position = params.text_document_position.position;

    let document = lsp.documents.get_document(&uri).context(format!(
        "Document '{:?}' not found in workspace",
        uri.as_str()
    ))?;

    // TODO: To completion completion, first get the trigger character if it exists in the
    // params.context Then track back from params position to find out what kind of completion is
    // this after tracking back if the input is `[[` then show file paths other wise if the
    // previous is a `#` then show headers for that specific file path that came before it, etc.

    // TODO: Get content and look back here, maybe refactor the function out so I can use it in
    // other places
    let slice = document.content.slice(..);

    Ok(Some(CompletionResponse::Array(vec![
        CompletionItem {
            label: "main.rs".to_string(),
            detail: Some("/src/main.rs".to_string()),
            kind: Some(CompletionItemKind::FILE),
            ..Default::default()
        },
        CompletionItem {
            label: "lib.rs".to_string(),
            detail: Some("/src/lib.rs".to_string()),
            kind: Some(CompletionItemKind::FILE),
            ..Default::default()
        },
        CompletionItem {
            label: "mod.rs".to_string(),
            detail: Some("/src/mod.rs".to_string()),
            kind: Some(CompletionItemKind::FILE),
            ..Default::default()
        },
    ])))
}

pub fn process_completion_resolve(
    lsp: &mut Server,
    item: CompletionItem,
) -> Result<CompletionItem> {
    log::debug!("Completion Resolve {:?}", item);
    Ok(CompletionItem {
        label: item.label,
        ..Default::default()
    })
}
