use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionParams, CompletionResponse,
    CompletionTriggerKind, Documentation,
};
use miette::{Context, Result};

use crate::{
    document::{references::ReferenceKind, Document},
    TextBufferConversions,
};

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

    // TODO: Make all outputs for paths and headers be normalized without spaces and symbols
    if let Some(context) = params.context {
        let completions = match context.trigger_kind {
            CompletionTriggerKind::INVOKED => handle_invoked_completion(lsp, document, position),
            CompletionTriggerKind::TRIGGER_CHARACTER => {
                handle_trigger_completion(lsp, document, position)
            }
            CompletionTriggerKind::TRIGGER_FOR_INCOMPLETE_COMPLETIONS => {
                log::error!("Completions for incomplete trigger is not implemented yet");
                None
            }
            _ => panic!("Unexpected completion trigger kind"),
        };

        return Ok(completions.map(Into::into));
    }

    Ok(None)
}

fn handle_invoked_completion(
    lsp: &Server,
    document: &Document,
    position: lsp_types::Position,
) -> Option<Vec<CompletionItem>> {
    let slice = document.content.slice(..);
    let byte_pos = slice.lsp_position_to_byte(position);

    None
}

fn handle_trigger_completion(
    lsp: &Server,
    document: &Document,
    position: lsp_types::Position,
) -> Option<Vec<CompletionItem>> {
    let slice = document.content.slice(..);
    let byte_pos = slice.lsp_position_to_byte(position);

    let mut completions: Vec<CompletionItem> = vec![];

    if let Some(trigger_context) = slice.get_byte_slice(byte_pos.saturating_sub(2)..byte_pos) {
        if trigger_context == "[[" || trigger_context == "](" {
            for doc in lsp.documents.get_documents() {
                // TODO: Instead of including all info here, maybe put that in the completion
                // resolve, But since I already get all documents here, is it worth making this
                // reponse smaller?
                completions.push(CompletionItem {
                    label: doc.uri.to_string(),
                    kind: Some(CompletionItemKind::FILE),
                    detail: Some("Document".to_owned()),
                    documentation: Some(Documentation::String(doc.content.to_string())),
                    ..Default::default()
                });
            }
        }
    }

    if let Some(trigger_context) = slice.get_byte_slice(byte_pos.saturating_sub(1)..byte_pos) {
        // TODO: Handle these another time
        // || trigger_context == ":"

        if trigger_context == "#" {
            for doc_ref in &document.references {
                if let ReferenceKind::Header { level, content } = &doc_ref.kind {
                    completions.push(CompletionItem {
                        label: content.to_owned(),
                        kind: Some(CompletionItemKind::REFERENCE),
                        ..Default::default()
                    });
                }
            }
        }
    }

    Some(completions)
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
