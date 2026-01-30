use lib_core::{
    document::{Document, references::ReferenceKind},
    get_document,
};

use lsp_types::{GotoDefinitionParams, GotoDefinitionResponse, Location, Range};
use miette::{Context, Result};

use crate::{
    handlers::link_resolver::resolve_target_uri, helpers::normalize_header_content, server::Server,
};

pub fn process_goto_definition(
    lsp: &mut Server,
    params: GotoDefinitionParams,
) -> Result<Option<GotoDefinitionResponse>> {
    let uri = params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    let document = get_document!(lsp, &uri);

    let reference = document
        .get_reference_at_position(position)
        .context("No reference found at cursor position")?;

    match &reference.kind {
        ReferenceKind::Link { target, header, .. }
        | ReferenceKind::WikiLink { target, header, .. } => {
            let (target_doc, range) = find_definition(lsp, document, target, header.as_deref())?;

            Ok(Some(GotoDefinitionResponse::Scalar(Location {
                uri: target_doc.uri.clone(),
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
    let target_uri = resolve_target_uri(lsp, document, target)?;
    let target_doc = get_document!(lsp, &target_uri);

    let Some(header_text) = header else {
        return Ok((target_doc, Range::default()));
    };

    let target_content = header_text.strip_prefix('#').unwrap_or(header_text);
    let normalized_target = normalize_header_content(target_content);

    let reference = target_doc.references.iter().find(|reference| {
        let ReferenceKind::Header { content, .. } = &reference.kind else {
            return false;
        };

        if content == target_content {
            return true;
        }

        let normalized_content = normalize_header_content(content);

        normalized_content == target_content || normalized_content == normalized_target
    });

    match reference {
        Some(reference) => Ok((target_doc, reference.range)),
        None => {
            log::warn!(
                "Header '#{}' not found in document '{}'. Falling back to file start.",
                target_content,
                target
            );
            Ok((target_doc, Range::default()))
        }
    }
}
