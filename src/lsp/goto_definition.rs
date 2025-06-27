use std::str::FromStr;

use crate::{
    document::{
        references::{ReferenceKind, TargetHeader},
        Document,
    },
    lsp::server::Server,
    get_document,
    path::combine_and_normalize,
};
use lsp_types::{GotoDefinitionParams, GotoDefinitionResponse, Location, Range, Uri};
use miette::{Context, Result};

pub fn process_goto_definition(
    lsp: &mut Server,
    params: GotoDefinitionParams,
) -> Result<Option<GotoDefinitionResponse>> {
    let uri = params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    let document = get_document!(lsp, &uri);

    let reference = document.get_reference_at_position(position);

    let Some(reference) = reference else {
        return Err(miette::miette!("Definition not found"));
    };

    match &reference.kind {
        ReferenceKind::Link { target, header, .. }
        | ReferenceKind::WikiLink { target, header, .. } => {
            let (document, range) =
                find_definition(lsp, document, target, header.as_ref().cloned())?;

            Ok(Some(GotoDefinitionResponse::from(Location {
                uri: document.uri.clone(),
                range,
            })))
        }
        _ => Ok(None),
    }
}

fn find_definition<'a>(
    lsp: &'a Server,
    document: &Document,
    target: &str,
    header: Option<TargetHeader>,
) -> Result<(&'a Document, Range)> {
    let file_path = combine_and_normalize(&document.uri, &Uri::from_str(target).unwrap())?;

    let document = get_document!(lsp, &file_path);

    for reference in &document.references {
        match &reference.kind {
            ReferenceKind::Header { content, .. } => {
                if header.is_none() || header.clone().unwrap().content == *content {
                    return Ok((document, reference.range));
                }
            }
            _ => {}
        }
    }

    Err(miette::miette!("Definition not found"))
}
