use lsp_types::{error_codes, Range, TextDocumentPositionParams};
use miette::{Context, IntoDiagnostic, Result};
use serde::{Deserialize, Serialize};

use crate::{
    lsp::server::LspServer,
    message::{Request, Response},
    Reference,
};

use super::helpers;

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
        Ok(result) => Response::from_ok(request.id, result),
        Err(e) => Response::from_error(request.id, error_codes::REQUEST_FAILED, e.to_string()),
    }
}

fn process_hover_internal(lsp: &mut LspServer, request: &Request) -> Result<HoverResponse> {
    let params: HoverParams = serde_json::from_value(request.params.clone())
        .into_diagnostic()
        .context("Failed to parse hover params")?;

    let uri = params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    let document = lsp
        .get_document(&uri)
        .context("Document should exist somewhere")?;

    let reference = document.find_reference_at_position(position);

    let (contents, range) =
        if let Some(Reference::Link(link) | Reference::WikiLink(link)) = reference {
            let contents = helpers::get_content(lsp, &link)?;
            let range = document.span_to_range(&link.span);
            (contents, Some(range))
        } else {
            ("".to_string(), None)
        };

    Ok(HoverResponse { contents, range })
}
