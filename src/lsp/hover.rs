use crate::{lsp::server::LspServer, Reference};
use lsp_types::{Hover, HoverContents, HoverParams, MarkupContent, MarkupKind};
use miette::{Context, Result};

use super::helpers;

pub fn process_hover(lsp: &mut LspServer, params: HoverParams) -> Result<Option<Hover>> {
    let uri = params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    let document = lsp
        .get_document(&uri)
        .context("Document should exist somewhere")?;

    let reference = document.find_reference_at_position(position);

    match reference {
        Some(Reference::Link(link) | Reference::WikiLink(link)) => {
            let contents = helpers::get_content(lsp, &link)?;
            let range = document.span_to_range(&link.span);
            Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: contents,
                }),
                range: Some(range),
            }))
        }
        _ => return Ok(None),
    }
}
