use lib_core::document::references::ReferenceKind;

use lsp_types::{Hover, HoverContents, HoverParams, MarkupContent, MarkupKind};
use miette::{Context, Result};
use tracing::debug;

use crate::{get_document, helpers::get_content, server_state::ServerState};

pub fn process_hover(lsp: &mut ServerState, params: HoverParams) -> Result<Option<Hover>> {
    let uri = params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    debug!("process_hover: uri={:?}, position={:?}", uri, position);

    let document = get_document!(lsp, &uri);

    let reference = document.get_reference_at_position(position);

    match reference {
        Some(reference) => match &reference.kind {
            ReferenceKind::Link { target, header, .. }
            | ReferenceKind::WikiLink { target, header, .. } => {
                debug!(
                    "Found Link/WikiLink reference: target={}, header={:?}",
                    target, header
                );
                let contents = get_content(lsp, document, target, header.as_deref())?;
                Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: contents,
                    }),
                    range: Some(reference.range),
                }))
            }
            kind => {
                debug!("Reference found but unsupported kind: {:?}", kind);
                Ok(None)
            }
        },
        None => {
            debug!("No reference found at position {:?}", position);
            Ok(None)
        }
    }
}
