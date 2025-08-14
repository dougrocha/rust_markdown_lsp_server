use lsp_types::{Location, ReferenceParams, Uri};
use miette::{Context, Result};

use crate::{
    document::{
        references::{Reference as DocReference, ReferenceKind},
        Document,
    },
    get_document,
    lsp::helpers::{normalize_header_content, resolve_target_uri},
};

use super::server::{DocumentStore, Server};

pub fn process_references(
    lsp: &mut Server,
    params: ReferenceParams,
) -> Result<Option<Vec<Location>>> {
    let uri = params.text_document_position.text_document.uri;
    let position = params.text_document_position.position;

    let document = get_document!(lsp, &uri);

    let Some(reference) = document.get_reference_at_position(position) else {
        log::error!("No reference found at position {position:?}");
        return Ok(Some(Vec::new()));
    };

    let mut ref_locations =
        ReferenceCollector::new(document, &uri, reference, lsp).collect_from(&lsp.documents);

    // Always put the source reference first in the list
    if params.context.include_declaration {
        let source_location = Location::new(uri.clone(), reference.range);
        ref_locations.insert(0, source_location);
    }

    Ok(Some(ref_locations))
}

struct ReferenceCollector<'a> {
    lsp: &'a Server,
    source_doc: &'a Document,
    source_uri: &'a Uri,
    source_ref: &'a DocReference,
}

impl<'a> ReferenceCollector<'a> {
    fn new(
        src_doc: &'a Document,
        uri: &'a lsp_types::Uri,
        reference: &'a DocReference,
        lsp: &'a Server,
    ) -> Self {
        Self {
            source_doc: src_doc,
            source_uri: uri,
            source_ref: reference,
            lsp,
        }
    }

    fn collect_from(&self, documents: &DocumentStore) -> Vec<Location> {
        documents
            .get_references_with_uri()
            .filter(|(uri, ref_doc)| !self.is_source_reference(uri, ref_doc))
            .filter_map(|(uri, ref_doc)| self.check_reference_match(uri, ref_doc))
            .collect()
    }

    fn is_source_reference(&self, uri: &lsp_types::Uri, reference: &DocReference) -> bool {
        uri == self.source_uri && reference.range == self.source_ref.range
    }

    fn check_reference_match(
        &self,
        uri: &lsp_types::Uri,
        reference: &DocReference,
    ) -> Option<Location> {
        match &self.source_ref.kind {
            ReferenceKind::Header { content, .. } => {
                self.match_header_reference(uri, reference, content)
            }
            ReferenceKind::Link { header, target, .. }
            | ReferenceKind::WikiLink { header, target, .. } => {
                let Ok(resolved_target) =
                    resolve_target_uri(self.source_doc, target, self.lsp.root())
                else {
                    log::error!("Check reference resolution failed");
                    return None;
                };

                self.match_link_reference(uri, reference, header.as_deref(), &resolved_target)
            }
        }
    }

    fn match_header_reference(
        &self,
        uri: &lsp_types::Uri,
        reference: &DocReference,
        source_content: &str,
    ) -> Option<Location> {
        let (link_header, link_target) = extract_link_parts(&reference.kind)?;
        let link_header = link_header?;

        let Ok(link_target) = resolve_target_uri(self.source_doc, link_target, self.lsp.root())
        else {
            log::error!("Header reference resolution failed");
            return None;
        };

        if *uri == link_target && headers_match(source_content, link_header) {
            Some(Location::new(uri.clone(), reference.range))
        } else {
            None
        }
    }

    fn match_link_reference(
        &self,
        uri: &lsp_types::Uri,
        reference: &DocReference,
        source_header: Option<&str>,
        source_target: &lsp_types::Uri,
    ) -> Option<Location> {
        let location = Location::new(uri.clone(), reference.range);

        match &reference.kind {
            ReferenceKind::Link { header, target, .. }
            | ReferenceKind::WikiLink { header, target, .. } => {
                let Ok(resolved_target) =
                    resolve_target_uri(self.source_doc, target, self.lsp.root())
                else {
                    log::error!("Link reference resolution failed");
                    return None;
                };

                if *source_target == resolved_target
                    && headers_are_compatible(source_header, header.as_deref())
                {
                    Some(location)
                } else {
                    None
                }
            }
            ReferenceKind::Header { content, .. } => source_header
                .filter(|sh| uri == source_target && headers_match(content, sh))
                .map(|_| location),
        }
    }
}

fn headers_match(content1: &str, content2: &str) -> bool {
    normalize_header_content(content1) == normalize_header_content(content2)
}

fn extract_link_parts(kind: &ReferenceKind) -> Option<(Option<&str>, &str)> {
    match kind {
        ReferenceKind::Link { header, target, .. }
        | ReferenceKind::WikiLink { header, target, .. } => Some((header.as_deref(), target)),
        _ => None,
    }
}

fn headers_are_compatible(h1: Option<&str>, h2: Option<&str>) -> bool {
    match (h1, h2) {
        (Some(header1), Some(header2)) => header1 == header2,
        (None, None) => true,
        _ => false,
    }
}
