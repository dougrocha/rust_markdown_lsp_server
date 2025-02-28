use std::{char, ops::Range};

use chumsky::{
    prelude::end,
    text::{self, newline, TextParser},
    Parser,
};
use log::info;
use serde::{Deserialize, Serialize};

use crate::{
    message::{Request, Response},
    parser::{link_parser, wikilink_parser, InlineMarkdown},
    LspServer, MarkdownLink,
};

use super::TextDocumentPositionParams;

#[derive(Deserialize, Debug)]
pub struct HoverParams {
    #[serde(flatten)]
    text_document_position_params: TextDocumentPositionParams,
}

#[derive(Serialize, Debug)]
pub struct HoverResponse {
    contents: String,
    range: Option<Range<usize>>,
}

fn find_link_at_position(
    document: &str,
    links: &[MarkdownLink],
    line_number: usize,
    character: usize,
) -> Option<String> {
    let line = document.lines().nth(line_number)?;

    // TODO: Before working on this, transition all files to use ropey this is because ropey makes
    // transition from (x,y) to byte range easy
    info!("Parsing line: {:?}", line);
    info!("File links: {:?}", links);

    None
}

pub fn process_hover(lsp: &mut LspServer, request: Request) -> Response {
    let params: HoverParams = serde_json::from_value(request.params).unwrap();

    info!("Hover Request Params: {:#?}", params);

    let uri = &params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    let lsp_doc = lsp.get_document(uri).unwrap();
    let lsp_doc_links = lsp.get_document_links(uri).unwrap();

    let link = find_link_at_position(lsp_doc, lsp_doc_links, position.line, position.character);
    info!("Link: {:#?}", link);

    let hover_response = HoverResponse {
        contents: link.unwrap_or_default(),
        range: None,
    };
    let result = serde_json::to_value(hover_response).unwrap();

    Response::new(request.id, Some(result))
}
