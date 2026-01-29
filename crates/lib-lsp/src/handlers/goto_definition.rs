use lib_core::{
    document::{Document, references::ReferenceKind},
    get_document,
};

use lsp_types::{GotoDefinitionParams, GotoDefinitionResponse, Location, Range};
use miette::{Context, Result};

use crate::{
    helpers::{normalize_header_content, resolve_target_uri},
    server::Server,
};

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
            let (document, range) = find_definition(lsp, document, target, header.as_deref())?;

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
    header: Option<&str>,
) -> Result<(&'a Document, Range)> {
    let file_path = resolve_target_uri(lsp, document, target)?;

    let document = get_document!(lsp, &file_path);

    let reference = document.references.iter().find(|reference| {
        let ReferenceKind::Header { content, .. } = &reference.kind else {
            return false;
        };

        if header.is_none() {
            return true;
        }

        let target_header = header.unwrap();
        let target_content = target_header.strip_prefix('#').unwrap_or(target_header);

        // Try multiple matching strategies:
        // 1. Exact match
        // 2. Normalized target vs original content
        // 3. Normalized target vs normalized content
        *content == target_content
            || normalize_header_content(content) == target_content
            || normalize_header_content(content) == normalize_header_content(target_content)
    });

    match reference {
        Some(reference) => Ok((document, reference.range)),
        None => Err(miette::miette!("Definition not found")),
    }
}
