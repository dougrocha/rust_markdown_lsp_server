use lib_core::{document::Document, path::slug::header_slug};

use crate::text_buffer_conversions::TextBufferConversions;

use gen_lsp_types::{
    CompletionItem, CompletionItemKind, CompletionItemLabelDetails, CompletionParams,
    CompletionResponse, CompletionTriggerKind, Documentation, Position, Uri,
};
use miette::{Context, Result, miette};

use crate::{
    get_document, handlers::link_resolver, helpers::generate_link_text, server_state::ServerState,
    uri::UriExt,
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
    Footnote { is_incomplete: bool },
    Tag,
}

impl CompletionIntent<'_> {
    fn from_position<'a>(document: &'a Document, byte_pos: usize) -> Option<CompletionIntent<'a>> {
        let slice = document.source.slice(..);

        if byte_pos >= 2 {
            let trigger = slice
                .get_byte_slice(byte_pos.saturating_sub(2)..byte_pos)
                .map(|s| s.as_str())?;

            if let Some(trigger) = trigger {
                if trigger == "[^" {
                    return Some(CompletionIntent::Footnote {
                        is_incomplete: !has_closing_bracket(document, byte_pos),
                    });
                }

                if let Some(link_type) = LinkType::detect(trigger) {
                    return Some(CompletionIntent::Document(LinkContext {
                        link_type,
                        is_incomplete: !has_closing_chars(document, byte_pos, link_type),
                    }));
                }
            }
        }

        if byte_pos >= 1 {
            let trigger = slice
                .get_byte_slice(byte_pos.saturating_sub(1)..byte_pos)
                .map(|s| s.as_str())?;

            if let Some(trigger) = trigger
                && trigger == "#"
            {
                let hash_pos = byte_pos.saturating_sub(1);

                if let Some((file_path, link_type)) =
                    extract_file_and_link_type_from_context(document, hash_pos)
                {
                    return Some(CompletionIntent::Header(HeaderContext {
                        file_path,
                        link_type,
                        is_incomplete: !has_closing_chars(document, byte_pos, link_type),
                    }));
                }

                if !is_start_of_line(document, hash_pos) {
                    return Some(CompletionIntent::Tag);
                }
            }
        }

        None
    }
}

/// Returns true if `byte_pos` is the first non-whitespace character on its
/// line (i.e. this is likely a heading marker, not an inline tag).
fn is_start_of_line(document: &Document, byte_pos: usize) -> bool {
    let slice = document.source.slice(..);
    let line_start = slice.byte_offset_to_position(byte_pos).line;
    let line_start_byte = slice.position_to_byte_offset(Position::new(line_start, 0));

    slice
        .get_byte_slice(line_start_byte..byte_pos)
        .map(|s| s.chars().all(|c| c == ' ' || c == '\t'))
        .unwrap_or(false)
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
    let uri = params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    let document = get_document!(lsp, &uri);

    let context = params
        .context
        .ok_or_else(|| miette!("Completion context does not exist"))?;

    // TODO: Make all outputs for paths and headers be normalized without spaces and symbols
    let completions = match context.trigger_kind {
        CompletionTriggerKind::Invoked => handle_invoked_completion(lsp, document, position),
        CompletionTriggerKind::TriggerCharacter => {
            handle_trigger_completion(lsp, document, position)
        }
        CompletionTriggerKind::TriggerForIncompleteCompletions => {
            tracing::error!("Completions for incomplete trigger is not implemented yet");
            None
        }
    };

    Ok(completions.map(Into::into))
}

fn handle_invoked_completion(
    lsp: &ServerState,
    document: &Document,
    position: Position,
) -> Option<Vec<CompletionItem>> {
    let slice = document.source.slice(..);
    let byte_pos = slice.position_to_byte_offset(position);

    let (anchor_idx, anchor_char) = find_byte_backwards_any(&slice, byte_pos, b"[(#:^\n")?;

    if anchor_char == b'\n' {
        return None;
    }

    let trigger_pos = anchor_idx + 1;
    let trigger_lsp_pos = slice.byte_offset_to_position(trigger_pos);

    tracing::debug!(
        "Invoked: {:?}",
        slice.get_slice(trigger_pos.saturating_sub(2)..trigger_pos.saturating_add(4))
    );

    handle_trigger_completion(lsp, document, trigger_lsp_pos)
}

fn handle_trigger_completion(
    lsp: &ServerState,
    document: &Document,
    position: Position,
) -> Option<Vec<CompletionItem>> {
    let slice = document.source.slice(..);
    let byte_pos = slice.position_to_byte_offset(position);

    let intent = CompletionIntent::from_position(document, byte_pos)?;

    match intent {
        CompletionIntent::Document(ctx) => complete_document_links(lsp, document, ctx),
        CompletionIntent::Header(ctx) => complete_headers(lsp, document, ctx),
        CompletionIntent::Footnote { is_incomplete } => {
            complete_footnotes(document, is_incomplete)
        }
        CompletionIntent::Tag => complete_tags(lsp),
    }
}

fn complete_tags(lsp: &ServerState) -> Option<Vec<CompletionItem>> {
    let mut names: Vec<String> = lsp
        .documents
        .iter()
        .flat_map(|doc| doc.tags().map(|tag| tag.name_str(&doc.source).to_string()))
        .collect();

    names.sort_unstable();
    names.dedup();

    let completions = names
        .into_iter()
        .map(|name| CompletionItem {
            label: name.clone(),
            kind: Some(CompletionItemKind::Constant),
            detail: Some("Tag".to_owned()),
            insert_text: Some(name),
            ..Default::default()
        })
        .collect();

    Some(completions)
}

fn complete_footnotes(document: &Document, is_incomplete: bool) -> Option<Vec<CompletionItem>> {
    let completions = document
        .footnote_definitions()
        .map(|def| {
            let identifier = def.identifier_str(&document.source);
            let content = def.content_str(&document.source);

            let insert_text = if is_incomplete {
                format!("{identifier}]")
            } else {
                identifier.to_string()
            };

            CompletionItem {
                label: identifier.to_string(),
                kind: Some(CompletionItemKind::Reference),
                detail: Some("Footnote".to_owned()),
                documentation: Some(Documentation::String(content.to_string())),
                insert_text: Some(insert_text),
                ..Default::default()
            }
        })
        .collect();

    Some(completions)
}

fn complete_document_links(
    lsp: &ServerState,
    document: &Document,
    ctx: LinkContext,
) -> Option<Vec<CompletionItem>> {
    let mut completions: Vec<CompletionItem> = vec![];

    let source_root = lsp.get_workspace_root_for_path(&document.path);

    for doc in lsp.documents.iter() {
        if doc.path == document.path {
            continue;
        }

        let Some(source_uri) = Uri::from_file_path(&document.path) else {
            continue;
        };
        let Some(doc_uri) = Uri::from_file_path(&doc.path) else {
            continue;
        };
        // Generate link text based on config
        let link_text =
            match generate_link_text(&lsp.config.links, &source_uri, &doc_uri, source_root) {
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
            kind: Some(CompletionItemKind::File),
            detail: Some("Document".to_owned()),
            documentation: Some(Documentation::String(format!(
                "Preview of {}:\n\n```markdown\n{}\n```",
                link_text,
                doc.source
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

    let source_root = lsp.get_workspace_root_for_path(&document.path);

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

    let file_path = file_uri.to_file_path()?;
    let ref_doc = lsp.documents.get_document(&file_path)?;
    for header in ref_doc.headers() {
        let content = header.content_str(&ref_doc.source);
        let level = header.level;
        let header_id = header_slug(&content);

        let label = content.to_string();

        let insert_text = ctx
            .link_type
            .format_completion(&header_id, ctx.is_incomplete);

        completions.push(CompletionItem {
            label,
            label_details: Some(CompletionItemLabelDetails {
                detail: None,
                description: Some(format!("H{level}")),
            }),
            kind: Some(CompletionItemKind::Reference),
            documentation: Some(Documentation::String(format!(
                "# {content}\n\nHeading level {level}\n\nLink: `{header_id}`"
            ))),
            insert_text: Some(insert_text),
            ..Default::default()
        });
    }

    Some(completions)
}

fn has_closing_bracket(document: &Document, byte_pos: usize) -> bool {
    let slice = document.source.slice(..);

    if document.get_reference_at_offset(byte_pos).is_some() {
        return true;
    }

    slice
        .get_byte_slice(byte_pos..byte_pos.saturating_add(1))
        .map(|s| s == "]")
        .unwrap_or(false)
}

fn has_closing_chars(document: &Document, byte_pos: usize, link_type: LinkType) -> bool {
    let slice = document.source.slice(..);

    if document.get_reference_at_offset(byte_pos).is_some() {
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
    let slice = document.source.slice(..);

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

#[cfg(test)]
mod tests {
    use gen_lsp_types::{
        CompletionContext, PartialResultParams, TextDocumentIdentifier,
        TextDocumentPositionParams, WorkDoneProgressParams,
    };

    use super::*;
    use crate::test_utils::TestWorkspace;

    #[test]
    fn header_completion_after_target_and_hash() {
        let mut ws = TestWorkspace::new();
        ws.add_file("/workspace/notes.md", 1, "[link](./target.md#")
            .add_file(
                "/workspace/target.md",
                1,
                "# First Header\n\n## Second Header",
            );

        let params = CompletionParams {
            context: Some(CompletionContext {
                trigger_kind: CompletionTriggerKind::TriggerCharacter,
                trigger_character: Some("#".to_string()),
            }),
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: "file:///workspace/notes.md".parse().unwrap(),
                },
                position: Position::new(0, 19),
            },
        };

        let response = process_completion(&mut ws.state, params).unwrap().unwrap();
        let CompletionResponse::CompletionItemList(items) = response else {
            panic!("expected a completion item list");
        };

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].label, "First Header");
        assert_eq!(items[1].label, "Second Header");
    }

    #[test]
    fn document_link_completion_excludes_current_document() {
        let mut ws = TestWorkspace::new();
        ws.add_file("/workspace/notes.md", 1, "[[")
            .add_file("/workspace/other.md", 1, "content");

        let params = CompletionParams {
            context: Some(CompletionContext {
                trigger_kind: CompletionTriggerKind::TriggerCharacter,
                trigger_character: Some("[".to_string()),
            }),
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: "file:///workspace/notes.md".parse().unwrap(),
                },
                position: Position::new(0, 2),
            },
        };

        let response = process_completion(&mut ws.state, params).unwrap().unwrap();
        let CompletionResponse::CompletionItemList(items) = response else {
            panic!("expected a completion item list");
        };

        assert_eq!(items.len(), 1);
        assert!(items.iter().all(|item| item.label != "notes"));
    }

    #[test]
    fn footnote_completion_after_caret() {
        let mut ws = TestWorkspace::new();
        ws.add_file(
            "/workspace/notes.md",
            1,
            "See[^\n\n[^one]: first\n[^two]: second",
        );

        let params = CompletionParams {
            context: Some(CompletionContext {
                trigger_kind: CompletionTriggerKind::TriggerCharacter,
                trigger_character: Some("^".to_string()),
            }),
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: "file:///workspace/notes.md".parse().unwrap(),
                },
                position: Position::new(0, 5),
            },
        };

        let response = process_completion(&mut ws.state, params).unwrap().unwrap();
        let CompletionResponse::CompletionItemList(items) = response else {
            panic!("expected a completion item list");
        };

        let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
        assert_eq!(labels, vec!["one", "two"]);
    }

    #[test]
    fn tag_completion_after_hash_lists_workspace_tags() {
        let mut ws = TestWorkspace::new();
        ws.add_file("/workspace/notes.md", 1, "today I worked on #")
            .add_file("/workspace/other.md", 1, "some #project/backend work");

        let params = CompletionParams {
            context: Some(CompletionContext {
                trigger_kind: CompletionTriggerKind::TriggerCharacter,
                trigger_character: Some("#".to_string()),
            }),
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: "file:///workspace/notes.md".parse().unwrap(),
                },
                position: Position::new(0, 19),
            },
        };

        let response = process_completion(&mut ws.state, params).unwrap().unwrap();
        let CompletionResponse::CompletionItemList(items) = response else {
            panic!("expected a completion item list");
        };

        let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
        assert_eq!(labels, vec!["project/backend"]);
    }

    #[test]
    fn hash_at_start_of_line_does_not_trigger_tag_completion() {
        let mut ws = TestWorkspace::new();
        ws.add_file("/workspace/notes.md", 1, "#");

        let params = CompletionParams {
            context: Some(CompletionContext {
                trigger_kind: CompletionTriggerKind::TriggerCharacter,
                trigger_character: Some("#".to_string()),
            }),
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: "file:///workspace/notes.md".parse().unwrap(),
                },
                position: Position::new(0, 1),
            },
        };

        let response = process_completion(&mut ws.state, params).unwrap();
        assert!(response.is_none());
    }
}
