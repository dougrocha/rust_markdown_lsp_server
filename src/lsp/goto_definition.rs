use crate::{
    document::{
        references::{combine_uri_and_relative_path, LinkData, Reference},
        Document,
    },
    lsp::server::LspServer,
    message::{error_codes, Request, Response},
};
use lsp_types::{uri::URI, Range, TextDocumentPositionParams};
use miette::{IntoDiagnostic, Result};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct GotoDefinitionParams {
    #[serde(flatten)]
    text_document_position_params: TextDocumentPositionParams,
}

#[derive(Serialize, Debug)]
pub struct GotoDefinitionResponse {
    uri: URI,
    range: Range,
}

pub fn process_goto_definition(lsp: &mut LspServer, request: Request) -> Response {
    match process_goto_definition_internal(lsp, &request) {
        Ok(result) => Response::new(request.id, result),
        Err(e) => Response::error(request.id, error_codes::INTERNAL_ERROR, e.to_string()),
    }
}

fn process_goto_definition_internal(
    lsp: &mut LspServer,
    request: &Request,
) -> Result<GotoDefinitionResponse> {
    let params: GotoDefinitionParams =
        serde_json::from_value(request.params.clone()).into_diagnostic()?;
    let URI(uri) = params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    let document = lsp
        .get_document(&uri)
        .ok_or_else(|| miette::miette!("Document not found"))?;

    let reference = document.find_reference_at_position(position);

    if let Some(Reference::Link(link) | Reference::WikiLink(link)) = &reference {
        let (document, span) = find_definition(lsp, &link)?;
        let range = document.span_to_range(span);

        Ok(GotoDefinitionResponse {
            uri: document.uri.clone(),
            range,
        })
    } else {
        Err(miette::miette!("Definition not found"))
    }
}

fn find_definition<'a>(
    lsp: &'a LspServer,
    link_data: &'a LinkData,
) -> Result<(&'a Document, &'a std::ops::Range<usize>)> {
    let file_path = combine_uri_and_relative_path(link_data)
        .ok_or_else(|| miette::miette!("Invalid file path"))?;

    let document = lsp
        .get_document(file_path.to_string_lossy())
        .ok_or_else(|| miette::miette!("Document not found"))?;

    for reference in &document.references {
        if let Reference::Header { content, span, .. } = reference {
            if link_data.header.is_none() || content == &link_data.header.clone().unwrap().content {
                return Ok((document, span));
            }
        }
    }

    Err(miette::miette!("Definition not found"))
}
