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
    let reference_at_position = document.get_reference_at_position(position);

    let mut reference_locations = if let Some(reference) = reference_at_position {
        ReferenceCollector::new(document, &uri, reference, lsp).collect_from(&lsp.documents)
    } else {
        ReferenceCollector::collect_file_references(document, &uri, lsp)
    };

    // Include the hovered reference itself if requested
    if params.context.include_declaration {
        if let Some(reference) = reference_at_position {
            let declaration_location = Location::new(uri.clone(), reference.range);
            reference_locations.insert(0, declaration_location);
        }
    }

    Ok(Some(reference_locations))
}

/// Helper for collecting references to a specific item in the document
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

    /// Collect all references that point to the a file, regardless of header
    fn collect_file_references(
        source_doc: &Document,
        source_uri: &Uri,
        lsp: &Server,
    ) -> Vec<Location> {
        lsp.documents
            .get_references_with_uri()
            .filter_map(|(uri, reference)| {
                // Only process links that have targets
                if let Some(target) = reference.kind.get_target() {
                    Self::resolve_and_check_target(source_doc, target, source_uri, lsp)
                        .map(|_| Location::new(uri.clone(), reference.range))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Resolve a target URI and check if it matches the source URI
    fn resolve_and_check_target(
        source_doc: &Document,
        target: &str,
        source_uri: &Uri,
        lsp: &Server,
    ) -> Option<()> {
        match resolve_target_uri(source_doc, target, lsp.root()) {
            Ok(resolved_target) if resolved_target == *source_uri => Some(()),
            Ok(_) => None,
            Err(err) => {
                log::error!("Target resolution failed: {:?}", err);
                None
            }
        }
    }

    fn is_source_reference(&self, uri: &lsp_types::Uri, reference: &DocReference) -> bool {
        uri == self.source_uri && reference.range == self.source_ref.range
    }

    /// Check if a reference matches our source reference and return its location if so
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
                // Resolve the source link's target to compare with other references
                let resolved_target =
                    match resolve_target_uri(self.source_doc, target, self.lsp.root()) {
                        Ok(target) => target,
                        Err(err) => {
                            log::error!("Source link target resolution failed: {:?}", err);
                            return None;
                        }
                    };

                self.match_link_reference(uri, reference, header.as_deref(), &resolved_target)
            }
        }
    }

    /// Find links that reference the given header
    fn match_header_reference(
        &self,
        uri: &lsp_types::Uri,
        reference: &DocReference,
        source_content: &str,
    ) -> Option<Location> {
        let target = reference.kind.get_target()?;

        Self::resolve_and_check_target(self.source_doc, target, self.source_uri, self.lsp)?;

        if let Some(link_header) = reference.kind.get_link_header() {
            if normalized_headers_match(source_content, link_header) {
                Some(Location::new(uri.clone(), reference.range))
            } else {
                None
            }
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
            ReferenceKind::Link { .. } | ReferenceKind::WikiLink { .. } => self
                .match_link_to_link(reference, source_header, source_target)
                .map(|_| location),
            ReferenceKind::Header { .. } => self
                .match_link_to_header(reference, uri, source_header, source_target)
                .map(|_| location),
        }
    }

    fn match_link_to_link(
        &self,
        reference: &DocReference,
        source_header: Option<&str>,
        source_target: &lsp_types::Uri,
    ) -> Option<()> {
        let target = reference.kind.get_target()?;

        Self::resolve_and_check_target(self.source_doc, target, source_target, self.lsp)?;

        let reference_header = reference.kind.get_link_header();

        if headers_are_compatible(source_header, reference_header) {
            Some(())
        } else {
            None
        }
    }

    fn match_link_to_header(
        &self,
        reference: &DocReference,
        uri: &lsp_types::Uri,
        source_header: Option<&str>,
        source_target: &lsp_types::Uri,
    ) -> Option<()> {
        if uri != source_target {
            return None;
        }

        let header_content = reference.kind.get_content()?;
        let source_header = source_header?;

        if normalized_headers_match(header_content, source_header) {
            Some(())
        } else {
            None
        }
    }
}

// This will normalize both headers before comparing
fn normalized_headers_match(content1: &str, content2: &str) -> bool {
    normalize_header_content(content1) == normalize_header_content(content2)
}

/// Headers are compatible if they are both None or both Some and equal
fn headers_are_compatible(h1: Option<&str>, h2: Option<&str>) -> bool {
    match (h1, h2) {
        (Some(header1), Some(header2)) => header1 == header2,
        (None, None) => true,
        _ => false,
    }
}
