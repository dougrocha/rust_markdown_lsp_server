use std::{
    char, fs,
    path::{Path, PathBuf},
};

use log::debug;
use serde::{Deserialize, Serialize};

use crate::{
    message::{Request, Response},
    LinkData, LspServer, Reference,
};

use super::{Range, TextDocumentPositionParams};

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
    let source_path = Path::new(link_data.file_name.trim_start_matches("file://"));
    let source_dir = source_path.parent()?;
    Some(source_dir.join(link_data.url.clone()))
}

fn find_reference_at_position<'a>(
    lsp: &'a LspServer,
    uri: &str,
    document: &str,
    line_number: usize,
    character: usize,
) -> Option<&'a LinkData> {
    let document_links = lsp.get_document_references(uri)?;

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

    let uri = &params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    let document = lsp.get_document(uri).unwrap();
    let link_data =
        find_reference_at_position(lsp, uri, document, position.line, position.character);

    let (contents, range) = if let Some(reference) = link_data {
        let contents = get_content(lsp, reference);
        let range = Range::from_span(document, reference.span.clone());

        (contents, Some(range))
    } else {
        (String::new(), None)
    };

    let hover_response = HoverResponse { contents, range };

    let result = serde_json::to_value(hover_response).unwrap();
    Response::new(request.id, Some(result))
}

fn get_content(lsp: &LspServer, link_data: &LinkData) -> String {
    let filepath = combine_uri_and_relative_path(link_data).unwrap_or_default();
    let file_contents = fs::read_to_string(filepath).unwrap_or_default();

    if let Some(header) = &link_data.header {
        // Maybe seperate header content and level
        let links = lsp.get_document_references(&link_data.url);
        debug!("Header: {:?}", header);
        debug!("Links: {:?}", links);

        // find matching header in file

        // TODO: Handle reading on the header section
        // But find some way to read the file
    }

    file_contents
}
