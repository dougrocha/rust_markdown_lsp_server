use std::str::FromStr;

use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionItemLabelDetails, CompletionParams,
    CompletionResponse, CompletionTriggerKind, Documentation, Uri,
};
use miette::{Context, Result};

use crate::{
    document::{references::ReferenceKind, Document},
    lsp::helpers::normalize_header_content,
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
            let is_wiki_link = trigger_context == "[[";
            for doc in lsp.documents.get_documents() {
                let Ok(relative_path) = find_relative_path(&document.uri, &doc.uri) else {
                    continue;
                };

                // Handle spaces in path
                let display_path = relative_path.clone();
                let insert_path = if relative_path.contains(' ') {
                    if is_wiki_link {
                        relative_path // Wiki links can handle spaces
                    } else {
                        relative_path.replace(' ', "%20") // URL encode spaces for markdown links
                    }
                } else {
                    relative_path
                };

                // Create completion with proper text edit
                let text_edit = if is_wiki_link {
                    format!("{}]]", insert_path)
                } else {
                    format!(
                        "{}{}",
                        insert_path,
                        if byte_pos == slice.len_bytes() {
                            ")"
                        } else {
                            ""
                        }
                    )
                };

                completions.push(CompletionItem {
                    label: display_path.clone(),
                    kind: Some(CompletionItemKind::FILE),
                    detail: Some("Document".to_owned()),
                    documentation: Some(Documentation::String(format!(
                        "Preview of {}:\n\n```markdown\n{}\n```",
                        display_path,
                        doc.content
                            .to_string()
                            .lines()
                            .take(10)
                            .collect::<Vec<_>>()
                            .join("\n")
                    ))),
                    insert_text: Some(text_edit),
                    ..Default::default()
                });
            }
        }
    }

    if let Some(trigger_context) = slice.get_byte_slice(byte_pos.saturating_sub(1)..byte_pos) {
        if trigger_context == ":" {
            // do nothing for now
        }

        if trigger_context == "#" {
            let (file_path, is_wiki_link) =
                extract_file_and_link_type_from_context(document, byte_pos)?;
            let file_uri_path = Uri::from_str(&file_path).ok()?;
            let file_uri = combine_and_normalize(&document.uri, &file_uri_path).ok()?;

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
                    let header_id = normalize_header_content(content);

                    let label = content.to_uppercase();

                    // Check if closing characters already exist after cursor
                    let slice = document.content.slice(..);
                    let has_closing = if is_wiki_link {
                        // Check for ]] after current position
                        slice
                            .get_byte_slice(byte_pos..byte_pos.saturating_add(2))
                            .map(|s| s == "]]")
                            .unwrap_or(false)
                    } else {
                        // Check for ) after current position
                        slice
                            .get_byte_slice(byte_pos..byte_pos.saturating_add(1))
                            .map(|s| s == ")")
                            .unwrap_or(false)
                    };

                    // Add appropriate ending based on link type, only if not already present
                    let insert_text = if has_closing {
                        header_id.clone()
                    } else if is_wiki_link {
                        format!("{}]]", header_id)
                    } else {
                        format!("{})", header_id)
                    };

                    completions.push(CompletionItem {
                        label,
                        label_details: Some(CompletionItemLabelDetails {
                            detail: None,
                            description: Some(format!("H{}", level)),
                        }),
                        kind: Some(CompletionItemKind::REFERENCE),
                        documentation: Some(Documentation::String(format!(
                            "# {}\n\nHeading level {}\n\nLink: `{}`",
                            content, level, header_id
                        ))),
                        insert_text: Some(insert_text),
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

pub fn extract_file_and_link_type_from_context(
    document: &Document,
    byte_pos: usize,
) -> Option<(String, bool)> {
    let content = document.content.slice(..);
    let start = byte_pos.saturating_sub(200);
    let search_slice = content.slice(start..byte_pos.min(content.len_bytes()));

    let mut bracket_pos = None;
    let mut is_wiki_link = false;

    for i in 0..search_slice.len_bytes().saturating_sub(1) {
        let window = &search_slice.byte_slice(i..i + 2);
        if window == "[[" {
            bracket_pos = Some(i);
            is_wiki_link = true;
        } else if window == "](" {
            bracket_pos = Some(i);
            is_wiki_link = false;
        }
    }

    let bracket_idx = bracket_pos?;
    let from_bracket = content.slice(start + bracket_idx + 2..);
    let hash_pos = from_bracket.bytes().position(|b| b == b'#')?;

    let file_bytes = &from_bracket.slice(..hash_pos);
    Some((file_bytes.to_string(), is_wiki_link))
}
