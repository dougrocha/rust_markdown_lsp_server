use crate::{
    document::{
        references::{combine_uri_and_relative_path, LinkData, Reference},
        Document,
    },
    lsp::server::LspServer,
};
use lsp_types::{GotoDefinitionParams, GotoDefinitionResponse, Location};
use miette::Result;

pub fn process_goto_definition(
    lsp: &mut LspServer,
    params: GotoDefinitionParams,
) -> Result<Option<GotoDefinitionResponse>> {
    let uri = params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    let document = lsp
        .get_document(&uri)
        .ok_or_else(|| miette::miette!("Document not found"))?;

    let reference = document.find_reference_at_position(position);

    if let Some(Reference::Link(link) | Reference::WikiLink(link)) = &reference {
        let (document, span) = find_definition(lsp, &link)?;
        let range = document.span_to_range(span);

        Ok(Some(GotoDefinitionResponse::from(Location {
            uri: document.uri.clone(),
            range,
        })))
    } else {
        Err(miette::miette!("Definition not found"))
    }
}

fn find_definition<'a>(
    lsp: &'a LspServer,
    link_data: &'a LinkData,
) -> Result<(&'a Document, &'a std::ops::Range<usize>)> {
    let file_path = combine_uri_and_relative_path(&link_data.source, &link_data.target)?;

    let document = lsp
        .get_document(&file_path)
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
