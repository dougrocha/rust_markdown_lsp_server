use lib_core::document::Reference;

use gen_lsp_types::{Contents, Hover, HoverParams, MarkupContent, MarkupKind};
use miette::{Context, Result};
use tracing::debug;

use crate::{
    get_document, helpers::get_content, server_state::ServerState,
    text_buffer_conversions::TextBufferConversions, uri::UriExt,
};

pub fn process_hover(lsp: &mut ServerState, params: HoverParams) -> Result<Option<Hover>> {
    let uri = params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    debug!("process_hover: uri={:?}, position={:?}", uri, position);

    let document = get_document!(lsp, &uri);

    let Some(offset) = document
        .source
        .slice(..)
        .try_position_to_byte_offset(position)
    else {
        return Ok(None);
    };

    let reference = document.get_reference_at_offset(offset);

    match reference {
        Some(Reference::Link(link)) => {
            let target = link.target_str(&document.source);
            let header = link.header_str(&document.source);

            debug!("Found link: target={}, header={:?}", target, header);

            let range = document
                .source
                .slice(..)
                .byte_to_lsp_range(link.span);

            let contents = get_content(lsp, document, &target, header.as_deref())?;

            Ok(Some(Hover {
                contents: Contents::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: contents,
                }),
                range: Some(range),
            }))
        }
        Some(Reference::Header(_)) => Ok(None),
        None => {
            debug!("No reference found at position {:?}", position);
            Ok(None)
        }
    }
}
