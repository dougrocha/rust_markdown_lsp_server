use std::{char, fs};

use miette::{Context, IntoDiagnostic, Result};
use serde::{Deserialize, Serialize};

use crate::{
    document::references::combine_uri_and_relative_path,
    message::{error_codes, Request, Response},
    LinkData, LinkHeader, LspServer, Reference,
};

use super::{Position, Range, TextDocumentPositionParams, URI};

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

pub fn process_hover(lsp: &mut LspServer, request: Request) -> Response {
    match process_hover_internal(lsp, &request) {
        Ok(result) => Response::new(request.id, result),
        Err(e) => Response::error(request.id, error_codes::INTERNAL_ERROR, e.to_string()),
    }
}

fn process_hover_internal(lsp: &mut LspServer, request: &Request) -> Result<HoverResponse> {
    let params: HoverParams = serde_json::from_value(request.params.clone())
        .into_diagnostic()
        .context("Parsing this tool's semver version failed.")?;

    let URI(uri) = params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    let document = lsp
        .get_document(&uri)
        .context("Document should exist somewhere")?;

    let link_data = document.find_reference_at_position(Position {
        line: position.line,
        character: position.character,
    });

    let (contents, range) = if let Some(reference) = link_data {
        let contents = get_content(lsp, reference)?;
        let range = Range::from_span("", &reference.span);

        (contents, Some(range))
    } else {
        (String::new(), None)
    };

    Ok(HoverResponse { contents, range })
}

fn get_content(lsp: &LspServer, link_data: &LinkData) -> Result<String> {
    let filepath = combine_uri_and_relative_path(link_data).unwrap_or_default();
    let file_contents = fs::read_to_string(filepath).unwrap_or_default();

    if link_data.header.is_none() || lsp.root.is_none() {
        return Ok(file_contents);
    }

    let header = link_data.header.as_ref().unwrap();
    let root = lsp.root.as_ref().unwrap();

    let joined_url = root.join(&link_data.url);
    let linked_url = joined_url.canonicalize().unwrap_or(joined_url);

    let linked_doc = lsp.get_document(linked_url.to_str().unwrap()).unwrap();
    let links = &linked_doc.references;

    let (start_index, end_index) = find_header_section(header, links);

    let extracted_content = match (start_index, end_index) {
        (Some(start), Some(end)) if start < end && end <= file_contents.len() => {
            &file_contents[start..end]
        }
        (Some(start), None) if start < file_contents.len() => &file_contents[start..],
        _ => &file_contents,
    };

    Ok(extracted_content.to_string())
}

fn find_header_section(header: &LinkHeader, links: &[Reference]) -> (Option<usize>, Option<usize>) {
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

    (start_index, end_index)
}
