use lib_core::{
    document::{Document, references::ReferenceKind},
    get_document,
    text_buffer_conversions::TextBufferConversions,
};

use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionItemLabelDetails, CompletionParams,
    CompletionResponse, CompletionTriggerKind, Documentation,
};
use miette::{Context, Result, miette};

use crate::{
    handlers::link_resolver,
    helpers::{self, normalize_header_content},
    server_state::ServerState,
};

pub mod completion_resolve;

#[derive(Debug, Clone, Copy, PartialEq)]
struct HeaderContext<'a> {
    file_path: &'a str,
    link_type: LinkType,
    is_incomplete: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct LinkContext {
    link_type: LinkType,
    is_incomplete: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CompletionIntent<'a> {
    Document(LinkContext),
    Header(HeaderContext<'a>),
    Footnote,
}

impl CompletionIntent<'_> {
    fn from_position<'a>(document: &'a Document, byte_pos: usize) -> Option<CompletionIntent<'a>> {
        let slice = document.content.slice(..);

        if byte_pos >= 2 {
            let trigger = slice
                .get_byte_slice(byte_pos.saturating_sub(2)..byte_pos)
                .map(|s| s.as_str())?;

            if let Some(trigger) = trigger
                && let Some(link_type) = LinkType::detect(trigger)
            {
                return Some(CompletionIntent::Document(LinkContext {
                    link_type,
                    is_incomplete: !has_closing_chars(document, byte_pos, link_type),
                }));
            }
        }

        if byte_pos >= 1 {
            let trigger = slice
                .get_byte_slice(byte_pos.saturating_sub(1)..byte_pos)
                .map(|s| s.as_str())?;

            if let Some(trigger) = trigger
                && trigger == "#"
            {
                let (file_path, link_type) =
                    extract_file_and_link_type_from_context(document, byte_pos.saturating_sub(1))?;

                return Some(CompletionIntent::Header(HeaderContext {
                    file_path,
                    link_type,
                    is_incomplete: !has_closing_chars(document, byte_pos, link_type),
                }));
            }
        }

        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum LinkType {
    WikiLink,
    MarkdownLink,
}

impl LinkType {
    fn detect(text: &str) -> Option<Self> {
        match text {
            "[[" => Some(Self::WikiLink),
            "](" => Some(Self::MarkdownLink),
            _ => None,
        }
    }

    fn suffix(&self) -> &'static str {
        match self {
            LinkType::WikiLink => "]]",
            LinkType::MarkdownLink => ")",
        }
    }

    fn format_completion(&self, text: &str, is_incomplete: bool) -> String {
        if is_incomplete {
            format!("{}{}", text, self.suffix())
        } else {
            text.to_string()
        }
    }

    fn encode_text(&self, text: &str) -> String {
        match self {
            // Wiki links typically handle spaces natively
            LinkType::WikiLink => text.to_string(),
            // Markdown links (URLs) require percent-encoding for spaces
            LinkType::MarkdownLink => text.replace(' ', "%20"),
        }
    }
}

pub fn process_completion(
    lsp: &mut ServerState,
    params: CompletionParams,
) -> Result<Option<CompletionResponse>> {
    let uri = params.text_document_position.text_document.uri;
    let position = params.text_document_position.position;

    let document = get_document!(lsp, &uri);

    let context = params
        .context
        .ok_or_else(|| miette!("Completion context does not exist"))?;

    // TODO: Make all outputs for paths and headers be normalized without spaces and symbols
    let completions = match context.trigger_kind {
        CompletionTriggerKind::INVOKED => handle_invoked_completion(lsp, document, position),
        CompletionTriggerKind::TRIGGER_CHARACTER => {
            handle_trigger_completion(lsp, document, position)
        }
        CompletionTriggerKind::TRIGGER_FOR_INCOMPLETE_COMPLETIONS => {
            tracing::error!("Completions for incomplete trigger is not implemented yet");
            None
        }
        _ => {
            return Err(miette!(
                "Unexpected completion trigger kind {:?}",
                context.trigger_kind
            ));
        }
    };

    Ok(completions.map(Into::into))
}

fn handle_invoked_completion(
    lsp: &ServerState,
    document: &Document,
    position: lsp_types::Position,
) -> Option<Vec<CompletionItem>> {
    let slice = document.content.slice(..);
    let byte_pos = slice.lsp_position_to_byte(position);

    let (anchor_idx, anchor_char) = find_byte_backwards_any(&slice, byte_pos, b"[(#:\n")?;

    if anchor_char == b'\n' {
        return None;
    }

    let trigger_pos = anchor_idx + 1;
    let trigger_lsp_pos = slice.byte_to_lsp_position(trigger_pos);

    tracing::debug!(
        "Invoked: {:?}",
        slice.get_slice(trigger_pos.saturating_sub(2)..trigger_pos.saturating_add(4))
    );

    handle_trigger_completion(lsp, document, trigger_lsp_pos)
}

fn handle_trigger_completion(
    lsp: &ServerState,
    document: &Document,
    position: lsp_types::Position,
) -> Option<Vec<CompletionItem>> {
    let slice = document.content.slice(..);
    let byte_pos = slice.lsp_position_to_byte(position);

    let intent = CompletionIntent::from_position(document, byte_pos)?;

    match intent {
        CompletionIntent::Document(ctx) => complete_document_links(lsp, document, ctx),
        CompletionIntent::Header(ctx) => complete_headers(lsp, document, ctx),
        CompletionIntent::Footnote => {
            // do nothing
            None
        }
    }
}

fn complete_document_links(
    lsp: &ServerState,
    document: &Document,
    ctx: LinkContext,
) -> Option<Vec<CompletionItem>> {
    let mut completions: Vec<CompletionItem> = vec![];

    let source_root = lsp.get_workspace_root_for_uri(&document.uri);

    for doc in lsp.documents.iter() {
        // Generate link text based on config
        let link_text = match helpers::generate_link_text(
            &lsp.config.links,
            &document.uri,
            &doc.uri,
            source_root,
        ) {
            Ok(text) => text,
            Err(e) => {
                tracing::warn!("Failed to generate link text: {}", e);
                continue;
            }
        };

        let encoded_text = ctx.link_type.encode_text(&link_text);

        let insert_text = ctx
            .link_type
            .format_completion(&encoded_text, ctx.is_incomplete);

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
    lsp: &ServerState,
    document: &Document,
    ctx: HeaderContext,
) -> Option<Vec<CompletionItem>> {
    let mut completions: Vec<CompletionItem> = vec![];

    let source_root = lsp.get_workspace_root_for_uri(&document.uri);

    let file_uri = match link_resolver::resolve_link(
        ctx.file_path,
        document,
        &lsp.config.links,
        &lsp.documents,
        source_root,
    ) {
        Ok(uri) => uri,
        Err(e) => {
            tracing::warn!(
                "Header completion failed to resolve link '{}': {}",
                ctx.file_path,
                e
            );
            return None;
        }
    };

    let ref_doc = lsp.documents.get_document(&file_uri)?;
    for doc_ref in &ref_doc.references {
        if let ReferenceKind::Header { level, content } = &doc_ref.kind {
            let header_id = normalize_header_content(content);

            let label = content.clone();

            let insert_text = ctx
                .link_type
                .format_completion(&header_id, ctx.is_incomplete);

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

fn has_closing_chars(document: &Document, byte_pos: usize, link_type: LinkType) -> bool {
    let slice = document.content.slice(..);

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

fn find_byte_backwards_any(
    content: &ropey::RopeSlice<'_>,
    start_pos: usize,
    stop_chars: &[u8],
) -> Option<(usize, u8)> {
    for (i, byte) in content.bytes_at(start_pos).reversed().enumerate() {
        if i > 50 {
            break;
        }
        if stop_chars.contains(&byte) {
            // Return both the index and WHICH character we hit
            return Some((start_pos.saturating_sub(i + 1), byte));
        }
    }

    None
}

fn extract_file_and_link_type_from_context(
    document: &Document,
    byte_pos: usize,
) -> Option<(&str, LinkType)> {
    let slice = document.content.slice(..);

    let (idx, found_char) = find_byte_backwards_any(&slice, byte_pos, b"[(")?;

    match found_char {
        b'[' => {
            // Peek left: is it another '['? -> WikiLink
            if idx > 0 && slice.byte(idx - 1) == b'[' {
                let path = slice.get_byte_slice(idx + 1..byte_pos)?.as_str()?;
                return Some((path, LinkType::WikiLink));
            }
        }
        b'(' => {
            // Peek left: is it a ']'? -> MarkdownLink [text](path)
            if idx > 0 && slice.byte(idx - 1) == b']' {
                let path = slice.get_byte_slice(idx + 1..byte_pos)?.as_str()?;
                return Some((path, LinkType::MarkdownLink));
            }
        }
        _ => {}
    }

    None
}
