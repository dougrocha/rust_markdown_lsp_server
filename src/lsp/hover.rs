use std::{
    char, fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    message::{Request, Response},
    LinkData, LspServer, Reference,
};

use super::{Range, TextDocumentPositionParams, URI};

#[derive(Deserialize, Debug)]
pub struct HoverParams {
    #[serde(flatten)]
    text_document_position_params: TextDocumentPositionParams,
}

#[derive(Serialize, Debug)]
pub struct HoverResponse {
    contents: String,
    range: Option<Range>,
}

fn combine_uri_and_relative_path(link_data: &LinkData) -> Option<PathBuf> {
    let source_dir = Path::new(&link_data.file_name).parent()?;
    Some(source_dir.join(&link_data.url))
}

fn find_reference_at_position<'a>(
    lsp: &'a LspServer,
    uri: &str,
    document: &str,
    line_number: usize,
    character: usize,
) -> Option<&'a LinkData> {
    let document_links = lsp.get_document_references(uri);

    let line_byte_idx = str_indices::lines::to_byte_idx(document, line_number);
    let line_str = document.lines().nth(line_number).unwrap_or("");
    let character_byte_pos = line_str
        .chars()
        .take(character)
        .map(|c| c.len_utf8())
        .sum::<usize>();
    let cursor_byte_pos = line_byte_idx + character_byte_pos;

    document_links.iter().find_map(|reference| {
        if let Reference::Link(data) | Reference::WikiLink(data) = reference {
            if data.span.contains(&cursor_byte_pos) {
                Some(data)
            } else {
                None
            }
        } else {
            None
        }
    })
}

pub fn process_hover(lsp: &mut LspServer, request: Request) -> Response {
    let params: HoverParams = serde_json::from_value(request.params).unwrap();

    let URI(uri) = params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    let document = lsp.get_document(&uri).unwrap();
    let link_data =
        find_reference_at_position(lsp, &uri, document, position.line, position.character);

    let (contents, range) = if let Some(reference) = link_data {
        let contents = get_content(lsp, reference);
        let range = Range::from_span(document, reference.span.clone());

        (contents, Some(range))
    } else {
        (String::new(), None)
    };

    let hover_response = HoverResponse { contents, range };

    let result = serde_json::to_value(hover_response).unwrap();
    Response::new(request.id, result)
}

fn get_content(lsp: &LspServer, link_data: &LinkData) -> String {
    let filepath = combine_uri_and_relative_path(link_data).unwrap_or_default();
    let file_contents = fs::read_to_string(filepath).unwrap_or_default();

    if link_data.header.is_none() || lsp.root.is_none() {
        return file_contents;
    }
    let header = link_data.header.as_ref().unwrap();
    let root = lsp.root.as_ref().unwrap();

    let joined_url = root.join(&link_data.url);
    let linked_url = joined_url.canonicalize().unwrap_or(joined_url);

    let links = lsp.get_document_references(linked_url.to_str().unwrap());

    let mut start_index = None;
    let mut end_index = None;

    for link in links {
        if let Reference::Header {
            level,
            content,
            span,
        } = link
        {
            if start_index.is_none() && *content == header.content && *level == header.level {
                start_index = Some(span.start);
                continue;
            } else if start_index.is_some() && *level <= header.level {
                end_index = Some(span.start);
                break;
            }
        }
    }

    // TODO: Handle reading on the header section
    // But find some way to read the file

    let extracted_content = match (start_index, end_index) {
        (Some(start), Some(end)) => &file_contents[start..end],
        (Some(start), None) => &file_contents[start..],
        _ => &file_contents,
    };

    extracted_content.to_string()
}
