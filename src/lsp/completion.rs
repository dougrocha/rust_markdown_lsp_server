use lsp_types::{CompletionItem, CompletionItemKind, CompletionParams, CompletionResponse};
use miette::Result;

use super::server::Server;

pub fn process_completion(
    lsp: &mut Server,
    params: CompletionParams,
) -> Result<Option<CompletionResponse>> {
    log::debug!("Completion params: {:?}", params);
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
