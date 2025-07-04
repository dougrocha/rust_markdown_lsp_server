use std::path::Path;

use lsp_types::{Location, ReferenceParams, Uri};
use miette::{Context, Result};

use crate::{
    document::references::{Reference as DocReference, ReferenceKind, TargetHeader},
    get_document,
    lsp::helpers::normalize_header_content,
    UriExt,
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
        log::debug!("No reference found at position {position:?}");
        return Ok(Some(Vec::new()));
    };

    let reference_collector = ReferenceCollector::new(&uri, reference);
    // Search for all references across all documents
    let mut ref_locations = reference_collector.collect_from(&lsp.documents);

    if params.context.include_declaration {
        ref_locations.push(Location::new(uri.clone(), reference.range));
    }

    Ok(Some(ref_locations))
}

struct ReferenceCollector<'a> {
    source_uri: &'a Uri,
    source_ref: &'a DocReference,
}

impl<'a> ReferenceCollector<'a> {
    fn new(uri: &'a lsp_types::Uri, reference: &'a DocReference) -> Self {
        Self {
            source_uri: uri,
            source_ref: reference,
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
                self.match_link_reference(uri, reference, header.as_ref(), target)
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

        if uri_matches_path(self.source_uri, link_target)
            && headers_match(source_content, &link_header.content)
        {
            Some(Location::new(uri.clone(), reference.range))
        } else {
            None
        }
    }

    fn match_link_reference(
        &self,
        uri: &lsp_types::Uri,
        reference: &DocReference,
        source_header: Option<&TargetHeader>,
        source_target: &str,
    ) -> Option<Location> {
        match &reference.kind {
            ReferenceKind::Link { header, target, .. }
            | ReferenceKind::WikiLink { header, target, .. } => {
                if source_target == target && headers_are_compatible(source_header, header.as_ref())
                {
                    Some(Location::new(uri.clone(), reference.range))
                } else {
                    None
                }
            }
            ReferenceKind::Header { content, .. } => {
                if let Some(source_header) = source_header {
                    if uri_matches_path(uri, source_target)
                        && headers_match(content, &source_header.content)
                    {
                        Some(Location::new(uri.clone(), reference.range))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        }
    }
}

fn headers_match(content1: &str, content2: &str) -> bool {
    normalize_header_content(content1) == normalize_header_content(content2)
}

fn extract_link_parts(kind: &ReferenceKind) -> Option<(Option<&TargetHeader>, &str)> {
    match kind {
        ReferenceKind::Link { header, target, .. }
        | ReferenceKind::WikiLink { header, target, .. } => Some((header.as_ref(), target)),
        _ => None,
    }
}

fn headers_are_compatible(h1: Option<&TargetHeader>, h2: Option<&TargetHeader>) -> bool {
    match (h1, h2) {
        (Some(header1), Some(header2)) => header1.content == header2.content,
        (None, None) => true,
        _ => false,
    }
}

fn uri_matches_path(uri: &lsp_types::Uri, path: &str) -> bool {
    let path_buf = Path::new(path).to_path_buf();

    let path_uri = lsp_types::Uri::from_file_path(path_buf);

    match path_uri {
        Some(converted_uri) => uri == &converted_uri,
        None => false,
    }
}
