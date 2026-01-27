use core::{document::references::ReferenceKind, get_document};

use lsp_types::{Hover, HoverContents, HoverParams, MarkupContent, MarkupKind};
use miette::{Context, Result};

use crate::{helpers::get_content, server::Server};

pub fn process_hover(lsp: &mut Server, params: HoverParams) -> Result<Option<Hover>> {
    let uri = params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    let document = get_document!(lsp, &uri);

    let reference = document.get_reference_at_position(position);

    match reference {
        Some(reference) => match &reference.kind {
            ReferenceKind::Link { target, header, .. }
            | ReferenceKind::WikiLink { target, header, .. } => {
                let contents = get_content(lsp, document, target, header.as_deref())?;
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
