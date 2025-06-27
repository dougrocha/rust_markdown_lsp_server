use crate::{document::references::ReferenceKind, get_document, lsp::server::Server};
use lsp_types::{Hover, HoverContents, HoverParams, MarkupContent, MarkupKind};
use miette::{Context, Result};

use super::helpers;

pub fn process_hover(lsp: &mut Server, params: HoverParams) -> Result<Option<Hover>> {
    let uri = params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    let document = get_document!(lsp, &uri);

    let reference = document.get_reference_at_position(position);

    match reference {
        Some(reference) => match &reference.kind {
            ReferenceKind::Link { target, header, .. }
            | ReferenceKind::WikiLink { target, header, .. } => {
                let contents = helpers::get_content(lsp, document, target, header.clone())?;
                Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: contents,
                    }),
                    range: Some(reference.range),
                }))
            }
            _ => Ok(None),
        },
        _ => Ok(None),
    }
}
