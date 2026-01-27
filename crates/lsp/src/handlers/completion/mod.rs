use core::{
    document::{Document, references::ReferenceKind},
    get_document,
    text_buffer_conversions::TextBufferConversions,
};

use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionItemLabelDetails, CompletionParams,
    CompletionResponse, CompletionTriggerKind, Documentation,
};
use miette::{Context, Result};

use crate::{
    handlers::link_resolver,
    helpers::{self, normalize_header_content},
    server::Server,
};

pub mod completion_resolve;

#[derive(Debug, Clone, Copy, PartialEq)]
enum LinkType {
    WikiLink,
    MarkdownLink,
}

pub fn process_completion(
    lsp: &mut Server,
    params: CompletionParams,
) -> Result<Option<CompletionResponse>> {
    let uri = params.text_document_position.text_document.uri;
    let position = params.text_document_position.position;

    let document = get_document!(lsp, &uri);

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

    log::error!("Context does not exist");

    Ok(None)
}

fn handle_invoked_completion(
    lsp: &Server,
    document: &Document,
    position: lsp_types::Position,
) -> Option<Vec<CompletionItem>> {
    let slice = document.content.slice(..);
    let byte_pos = slice.lsp_position_to_byte(position);

    let mut tmp_byte_pos = byte_pos.saturating_sub(1);
    while tmp_byte_pos > 0 {
        let byte = slice.byte(tmp_byte_pos);

        match byte {
            b'(' | b'[' => {
                break;
            }
            b')' | b']' => {
                log::debug!("Found closing bracket during backtrack, aborting completion");
                return None;
            }
            b'\n' => {
                log::debug!("Found newline during backtrack, no trigger found");
                return None;
            }
            _ => {
                tmp_byte_pos = tmp_byte_pos.saturating_sub(1);
            }
        }
    }
    tmp_byte_pos += 1;

    handle_trigger_completion(lsp, document, slice.byte_to_lsp_position(tmp_byte_pos))
}

fn handle_trigger_completion(
    lsp: &Server,
    document: &Document,
    position: lsp_types::Position,
) -> Option<Vec<CompletionItem>> {
    let slice = document.content.slice(..);
    let byte_pos = slice.lsp_position_to_byte(position);

    log::debug!(
        "Handling trigger completion: {:?}",
        slice.get_byte_slice(byte_pos.saturating_sub(4)..byte_pos)
    );

    if let Some(trigger_context) = slice.get_byte_slice(byte_pos.saturating_sub(2)..byte_pos)
        && (trigger_context == "[[" || trigger_context == "](")
    {
        let link_type = if trigger_context == "[[" {
            LinkType::WikiLink
        } else {
            LinkType::MarkdownLink
        };

        return complete_document_links(lsp, document, &slice, byte_pos, link_type);
    }

    if let Some(trigger_context) = slice.get_byte_slice(byte_pos.saturating_sub(1)..byte_pos) {
        if trigger_context == ":" {
            // do nothing for now
        }

        if trigger_context == "#" {
            return complete_headers(lsp, document, byte_pos);
        }
    }

    None
}

fn complete_document_links(
    lsp: &Server,
    document: &Document,
    slice: &ropey::RopeSlice<'_>,
    byte_pos: usize,
    link_type: LinkType,
) -> Option<Vec<CompletionItem>> {
    let mut completions: Vec<CompletionItem> = vec![];

    for doc in lsp.documents.get_documents() {
        // Generate link text based on config
        let link_text = match helpers::generate_link_text(
            &lsp.config.links,
            &document.uri,
            &doc.uri,
            lsp.root(),
        ) {
            Ok(text) => text,
            Err(e) => {
                log::warn!("Failed to generate link text: {}", e);
                continue;
            }
        };

        // Handle spaces in generated text
        let insert_text = if link_text.contains(' ') {
            match link_type {
                LinkType::WikiLink => link_text.clone(), // Wiki links handle spaces
                LinkType::MarkdownLink => link_text.replace(' ', "%20"), // URL encode for markdown links
            }
        } else {
            link_text.clone()
        };

        let position = slice.byte_to_lsp_position(byte_pos);
        log::debug!("Position: {:#?}", position);

        let has_closing = has_closing_chars(document, byte_pos, link_type, slice);

        let insert_text = if has_closing {
            insert_text.clone()
        } else {
            match link_type {
                LinkType::WikiLink => format!("{insert_text}]]"),
                LinkType::MarkdownLink => format!("{insert_text})"),
            }
        };

        completions.push(CompletionItem {
            label: link_text.clone(),
            kind: Some(CompletionItemKind::FILE),
            detail: Some("Document".to_owned()),
            documentation: Some(Documentation::String(format!(
                "Preview of {}:\n\n```markdown\n{}\n```",
                link_text,
                doc.content
                    .to_string()
                    .lines()
                    .take(10)
                    .collect::<Vec<_>>()
                    .join("\n")
            ))),
            insert_text: Some(insert_text),
            ..Default::default()
        });
    }

    Some(completions)
}

fn complete_headers(
    lsp: &Server,
    document: &Document,
    byte_pos: usize,
) -> Option<Vec<CompletionItem>> {
    let mut completions: Vec<CompletionItem> = vec![];

    let (file_path, link_type) = extract_file_and_link_type_from_context(document, byte_pos)?;
    let file_uri = match link_resolver::resolve_link(
        &file_path,
        document,
        &lsp.config.links,
        &lsp.documents,
        lsp.root(),
    ) {
        Ok(uri) => uri,
        Err(e) => {
            log::warn!(
                "Header completion failed to resolve link '{}': {}",
                file_path,
                e
            );
            return None;
        }
    };
    let ref_doc = lsp.documents.get_document(&file_uri)?;
    for doc_ref in &ref_doc.references {
        if let ReferenceKind::Header { level, content } = &doc_ref.kind {
            let header_id = normalize_header_content(content);

            let label = content.to_uppercase();
            let slice = document.content.slice(..);

            let has_closing = has_closing_chars(document, byte_pos, link_type, &slice);

            let insert_text = if has_closing {
                header_id.clone()
            } else {
                match link_type {
                    LinkType::WikiLink => format!("{header_id}]]"),
                    LinkType::MarkdownLink => format!("{header_id})"),
                }
            };

            completions.push(CompletionItem {
                label,
                label_details: Some(CompletionItemLabelDetails {
                    detail: None,
                    description: Some(format!("H{level}")),
                }),
                kind: Some(CompletionItemKind::REFERENCE),
                documentation: Some(Documentation::String(format!(
                    "# {content}\n\nHeading level {level}\n\nLink: `{header_id}`"
                ))),
                insert_text: Some(insert_text),
                ..Default::default()
            });
        }
    }

    Some(completions)
}

fn has_closing_chars(
    document: &Document,
    byte_pos: usize,
    link_type: LinkType,
    slice: &ropey::RopeSlice<'_>,
) -> bool {
    if document
        .get_reference_at_position(slice.byte_to_lsp_position(byte_pos))
        .is_some()
    {
        return true;
    }

    match link_type {
        LinkType::WikiLink => slice
            .get_byte_slice(byte_pos..byte_pos.saturating_add(2))
            .map(|s| s == "]]")
            .unwrap_or(false),
        LinkType::MarkdownLink => slice
            .get_byte_slice(byte_pos..byte_pos.saturating_add(1))
            .map(|s| s == ")")
            .unwrap_or(false),
    }
}

fn extract_file_and_link_type_from_context(
    document: &Document,
    byte_pos: usize,
) -> Option<(String, LinkType)> {
    let content = document.content.slice(..);
    let start = byte_pos.saturating_sub(200);
    let search_slice = content.slice(start..byte_pos.min(content.len_bytes()));

    let mut bracket_pos = None;
    let mut link_type: Option<LinkType> = None;

    for i in 0..search_slice.len_bytes().saturating_sub(1) {
        let window = &search_slice.byte_slice(i..i + 2);
        if window == "[[" {
            bracket_pos = Some(i);
            link_type = Some(LinkType::WikiLink);
        } else if window == "](" {
            bracket_pos = Some(i);
            link_type = Some(LinkType::MarkdownLink);
        }
    }

    let bracket_idx = bracket_pos?;
    let from_bracket = content.slice(start + bracket_idx + 2..);
    let hash_pos = from_bracket.bytes().position(|b| b == b'#')?;

    let file_bytes = &from_bracket.slice(..hash_pos);
    Some((file_bytes.to_string(), link_type?))
}
