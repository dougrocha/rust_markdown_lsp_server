use std::str::FromStr;

use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionItemLabelDetails, CompletionParams,
    CompletionResponse, CompletionTriggerKind, Documentation, Uri,
};
use miette::{Context, Result};

use crate::{
    document::{references::ReferenceKind, Document},
    path::{combine_and_normalize, find_relative_path},
    TextBufferConversions,
};

use super::server::Server;

pub fn process_completion(
    lsp: &mut Server,
    params: CompletionParams,
) -> Result<Option<CompletionResponse>> {
    let uri = params.text_document_position.text_document.uri;
    let position = params.text_document_position.position;

    let document = lsp.documents.get_document(&uri).context(format!(
        "Document '{:?}' not found in workspace",
        uri.as_str()
    ))?;

    // TODO: Make all outputs for paths and headers be normalized without spaces and symbols
    if let Some(context) = params.context {
        let completions = match context.trigger_kind {
            CompletionTriggerKind::INVOKED => {
                log::debug!("Handling invoked completion: {:?}", position);
                handle_invoked_completion(lsp, document, position)
            }
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

    log::error!("Context does not exist");

    Ok(None)
}

fn handle_invoked_completion(
    _lsp: &Server,
    document: &Document,
    position: lsp_types::Position,
) -> Option<Vec<CompletionItem>> {
    let slice = document.content.slice(..);
    let _byte_pos = slice.lsp_position_to_byte(position);

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

    log::debug!(
        "Handling trigger completion: {:?}",
        slice.get_byte_slice(byte_pos.saturating_sub(4)..byte_pos)
    );

    if let Some(trigger_context) = slice.get_byte_slice(byte_pos.saturating_sub(2)..byte_pos) {
        if trigger_context == "[[" || trigger_context == "](" {
            for doc in lsp.documents.get_documents() {
                let Ok(relative_path) = find_relative_path(&document.uri, &doc.uri) else {
                    continue;
                };

                // TODO: Instead of including all info here, maybe put that in the completion
                // resolve, But since I already get all documents here, is it worth making this
                // reponse smaller?
                completions.push(CompletionItem {
                    label: relative_path,
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
            // TODO create a variable where it looks back and grabs the file path of the current context

            let file_path = Uri::from_str(&extract_file_from_context(document, byte_pos)?).ok()?;
            let file_uri = combine_and_normalize(&document.uri, &file_path).ok()?;

            let ref_doc = lsp
                .documents
                .get_document(&file_uri)
                .context(format!(
                    "Document '{:?}' not found in workspace",
                    file_uri.as_str()
                ))
                .ok()?;

            for doc_ref in &ref_doc.references {
                if let ReferenceKind::Header { level, content } = &doc_ref.kind {
                    // TODO: Add proper text edits, spaces cannot go in to label
                    completions.push(CompletionItem {
                        label: content.to_owned(),
                        label_details: Some(CompletionItemLabelDetails {
                            detail: None,
                            description: Some(format!("H{}", level)),
                        }),
                        kind: Some(CompletionItemKind::FILE),
                        documentation: Some(Documentation::String(content.to_string())),
                        ..Default::default()
                    });
                }
            }
        }
    }

    log::debug!(
        "Completions labels: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );

    Some(completions)
}

pub fn process_completion_resolve(
    _lsp: &mut Server,
    item: CompletionItem,
) -> Result<CompletionItem> {
    // log::debug!("Completion Resolve {:#?}", item);
    Ok(CompletionItem {
        label: item.label,
        ..Default::default()
    })
}

pub fn extract_file_from_context(document: &Document, byte_pos: usize) -> Option<String> {
    let content = document.content.slice(..);
    let start = byte_pos.saturating_sub(200);
    let search_slice = content.slice(start..byte_pos.min(content.len_bytes()));

    let mut bracket_pos = None;
    for i in 0..search_slice.len_bytes().saturating_sub(1) {
        let window = &search_slice.byte_slice(i..i + 2);
        if window == "[[" || window == "](" {
            bracket_pos = Some(i);
        }
    }

    let bracket_idx = bracket_pos?;
    let from_bracket = content.slice(start + bracket_idx + 2..);
    let hash_pos = from_bracket.bytes().position(|b| b == b'#')?;

    let file_bytes = &from_bracket.slice(..hash_pos);
    Some(file_bytes.to_string())
}
