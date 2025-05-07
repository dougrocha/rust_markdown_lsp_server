use miette::{Context, IntoDiagnostic, Result};
use serde::{Deserialize, Serialize};

use crate::{
    document::references::combine_uri_and_relative_path,
    lsp::server::LspServer,
    message::{error_codes, Request, Response},
    LinkData, LinkHeader, Reference,
};

use super::{Position, Range, TextDocumentPositionParams};

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
        .context("Failed to parse hover params")?;

    let uri = params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    let document = lsp
        .get_document(uri)
        .context("Document should exist somewhere")?;

    let link_data = document.find_reference_at_position(Position {
        line: position.line,
        character: position.character,
    });

    let (contents, range) = if let Some(reference) = link_data {
        let contents = get_content(lsp, reference)?;
        let range = document.span_to_range(&reference.span);

        (contents, Some(range))
    } else {
        ("".to_string(), None)
    };

    Ok(HoverResponse { contents, range })
}

/// Retrieves the content from a linked document based on the provided link data.
fn get_content(lsp: &LspServer, link_data: &LinkData) -> Result<String> {
    let filepath = combine_uri_and_relative_path(link_data)
        .context("Failed to combine URI and relative path")?;
    let document = lsp
        .get_document(filepath.to_string_lossy())
        .ok_or_else(|| miette::miette!("Document not found"))?;
    let file_contents = document.content.slice(..);

    if link_data.header.is_none() {
        return Ok(file_contents.to_string());
    }

    let header = link_data.header.as_ref().unwrap();

    let linked_doc = lsp
        .get_document(filepath.to_string_lossy())
        .context("Linked document not found")?;
    let links = &linked_doc.references;

    let (start_index, end_index) = extract_header_section(header, links);

    let extracted_content = match (start_index, end_index) {
        (Some(start), Some(end)) if start < end && end <= file_contents.len_bytes() => {
            file_contents.byte_slice(start..end).to_string()
        }
        (Some(start), None) if start < file_contents.len_bytes() => {
            file_contents.byte_slice(start..).to_string()
        }
        _ => file_contents.to_string(),
    };

    Ok(extracted_content)
}

/// Extracts the start and end indices of a header section from the provided links.
fn extract_header_section(
    header: &LinkHeader,
    links: &[Reference],
) -> (Option<usize>, Option<usize>) {
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
