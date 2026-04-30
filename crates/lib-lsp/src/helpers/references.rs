use lib_core::{
    document::{
        Document,
        references::{Reference as DocReference, ReferenceKind},
    },
    vault::Vault,
};
use lsp_types::{Location, Uri};

use crate::{ServerState, handlers::link_resolver::resolve_target_uri, helpers::header_slug};

/// Helper for collecting references to a specific item in the document
pub(crate) struct ReferenceCollector<'a> {
    pub(crate) lsp: &'a ServerState,
    pub(crate) source_doc: &'a Document,
    pub(crate) source_uri: &'a Uri,
    pub(crate) source_ref: &'a DocReference,
}

impl<'a> ReferenceCollector<'a> {
    pub(crate) fn new(
        src_doc: &'a Document,
        uri: &'a lsp_types::Uri,
        reference: &'a DocReference,
        lsp: &'a ServerState,
    ) -> Self {
        Self {
            source_doc: src_doc,
            source_uri: uri,
            source_ref: reference,
            lsp,
        }
    }

    pub(crate) fn collect_from(&self, documents: &Vault) -> Vec<Location> {
        documents
            .get_references_with_uri()
            .filter(|(uri, ref_doc)| !self.is_source_reference(uri, ref_doc))
            .filter_map(|(uri, ref_doc)| self.check_reference_match(uri, ref_doc))
            .collect()
    }

    /// Collect all references that point to the a file, reaardless of header
    pub(crate) fn collect_file_reference_locations(
        lsp: &'a ServerState,
        source_uri: &'a Uri,
    ) -> Vec<Location> {
        Self::collect_file_references(lsp, source_uri)
            .map(|(uri, reference)| Location::new(uri.clone(), reference.range))
            .collect()
    }

    /// Collect all references that point to the a file, reaardless of header
    pub(crate) fn collect_file_references(
        lsp: &'a ServerState,
        source_uri: &'a Uri,
    ) -> impl Iterator<Item = (&'a Uri, &'a DocReference)> + 'a {
        lsp.documents
            .get_references_with_uri()
            .filter(move |(uri, reference)| {
                let Some(referring_doc) = lsp.documents.get_document(uri) else {
                    return false;
                };

                reference
                    .kind
                    .get_target()
                    .and_then(|target| {
                        Self::resolve_and_check_target(lsp, referring_doc, target, source_uri)
                    })
                    .is_some()
            })
    }

    /// Resolve a target URI and check if it matches the source URI
    pub(crate) fn resolve_and_check_target(
        lsp: &ServerState,
        source_doc: &Document,
        target: &str,
        source_uri: &Uri,
    ) -> Option<()> {
        match resolve_target_uri(lsp, source_doc, target) {
            Ok(resolved_target) if resolved_target == *source_uri => Some(()),
            Ok(_) => None,
            Err(err) => {
                tracing::error!("Target resolution failed: {:?}", err);
                None
            }
        }
    }

    pub(crate) fn is_source_reference(
        &self,
        uri: &lsp_types::Uri,
        reference: &DocReference,
    ) -> bool {
        uri == self.source_uri && reference.range == self.source_ref.range
    }

    /// Check if a reference matches our source reference and return its location if so
    pub(crate) fn check_reference_match(
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
                let resolved_target = match resolve_target_uri(self.lsp, self.source_doc, target) {
                    Ok(target) => target,
                    Err(err) => {
                        tracing::error!("Source link target resolution failed: {:?}", err);
                        return None;
                    }
                };

                self.match_link_reference(uri, reference, header.as_deref(), &resolved_target)
            }
        }
    }

    /// Find links that reference the given header
    pub(crate) fn match_header_reference(
        &self,
        uri: &lsp_types::Uri,
        reference: &DocReference,
        source_content: &str,
    ) -> Option<Location> {
        let target = reference.kind.get_target()?;

        Self::resolve_and_check_target(self.lsp, self.source_doc, target, self.source_uri)?;

        if let Some(link_header) = reference.kind.get_link_header()
            && normalized_headers_match(source_content, link_header)
        {
            Some(Location::new(uri.clone(), reference.range))
        } else {
            None
        }
    }

    pub(crate) fn match_link_reference(
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

    pub(crate) fn match_link_to_link(
        &self,
        reference: &DocReference,
        source_header: Option<&str>,
        source_target: &lsp_types::Uri,
    ) -> Option<()> {
        let target = reference.kind.get_target()?;

        Self::resolve_and_check_target(self.lsp, self.source_doc, target, source_target)?;

        let reference_header = reference.kind.get_link_header();

        if source_header == reference_header {
            Some(())
        } else {
            None
        }
    }

    pub(crate) fn match_link_to_header(
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
pub(crate) fn normalized_headers_match(content1: &str, content2: &str) -> bool {
    header_slug(content1) == header_slug(content2)
}
