use lib_core::{
    document::{Document, references::ReferenceKind},
    path::slug::header_slug,
};

use gen_lsp_types::{Definition, DefinitionParams, DefinitionResponse, Location, Range};
use miette::{Context, Result, miette};

use crate::{
    get_document, handlers::link_resolver::resolve_target_uri, server_state::ServerState,
    uri::UriExt,
};

pub fn process_goto_definition(
    lsp: &mut ServerState,
    params: DefinitionParams,
) -> Result<Option<DefinitionResponse>> {
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

            let target_uri = UriExt::from_file_path(&target_doc.path)
                .ok_or_else(|| miette!("Failed to convert path to URI: {:?}", target_doc.path))?;

            Ok(Some(DefinitionResponse::Definition(Definition::Location(
                Location {
                    uri: target_uri,
                    range,
                },
            ))))
        }
        _ => Ok(None),
    }
}

fn find_definition<'a>(
    lsp: &'a ServerState,
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
    let normalized_target = header_slug(target_content);

    let reference = target_doc.references.iter().find(|reference| {
        let ReferenceKind::Header { content, .. } = &reference.kind else {
            return false;
        };

        if content == target_content {
            return true;
        }

        let normalized_content = header_slug(content);

        normalized_content == target_content || normalized_content == normalized_target
    });

    match reference {
        Some(reference) => Ok((target_doc, reference.range)),
        None => {
            tracing::warn!(
                "Header '#{}' not found in document '{}'. Falling back to file start.",
                target_content,
                target
            );
            Ok((target_doc, Range::default()))
        }
    }
}
