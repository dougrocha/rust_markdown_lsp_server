use gen_lsp_types::{Location, Uri};
use lib_core::{
    document::{Document, Reference, index::Header, index::Link},
    vault::Vault,
};

use crate::{
    ServerState, handlers::link_resolver::resolve_target_uri,
    helpers::header_slug, text_buffer_conversions::TextBufferConversions, uri::UriExt,
};

/// Helper for collecting references to a specific item in the document
pub(crate) struct ReferenceCollector<'a> {
    pub(crate) lsp: &'a ServerState,
    pub(crate) source_doc: &'a Document,
    pub(crate) source_uri: &'a Uri,
    pub(crate) source_ref: Reference<'a>,
}

impl<'a> ReferenceCollector<'a> {
    pub(crate) fn new(
        src_doc: &'a Document,
        uri: &'a gen_lsp_types::Uri,
        reference: Reference<'a>,
        lsp: &'a ServerState,
    ) -> Self {
        Self {
            source_doc: src_doc,
            source_uri: uri,
            source_ref: reference,
            lsp,
        }
    }

    pub(crate) fn collect_from(&self, documents: &'a Vault) -> Vec<Location> {
        documents
            .iter()
            .filter_map(|doc| Some((Uri::from_file_path(&doc.path)?, doc)))
            .flat_map(|(uri, doc)| self.check_document(&uri, doc))
            .collect()
    }

    fn check_document(&self, uri: &Uri, doc: &'a Document) -> Vec<Location> {
        let links_uri = uri.clone();
        let links = doc.links().filter_map(move |link| {
            let reference = Reference::Link(link);
            if self.is_source_reference(&links_uri, reference) {
                return None;
            }
            self.check_reference_match(&links_uri, doc, reference)
        });

        let headers_uri = uri.clone();
        let headers = doc.headers().filter_map(move |header| {
            let reference = Reference::Header(header);
            if self.is_source_reference(&headers_uri, reference) {
                return None;
            }
            self.check_reference_match(&headers_uri, doc, reference)
        });

        let footnotes_uri = uri.clone();
        let footnote_refs = doc.footnote_references().filter_map(move |footnote_ref| {
            let reference = Reference::FootnoteRef(footnote_ref);
            if self.is_source_reference(&footnotes_uri, reference) {
                return None;
            }
            self.check_reference_match(&footnotes_uri, doc, reference)
        });

        let footnote_defs_uri = uri.clone();
        let footnote_defs = doc.footnote_definitions().filter_map(move |footnote_def| {
            let reference = Reference::FootnoteDef(footnote_def);
            if self.is_source_reference(&footnote_defs_uri, reference) {
                return None;
            }
            self.check_reference_match(&footnote_defs_uri, doc, reference)
        });

        links
            .chain(headers)
            .chain(footnote_refs)
            .chain(footnote_defs)
            .collect()
    }

    /// Collect all references that point to the a file, regardless of header
    pub(crate) fn collect_file_reference_locations(
        lsp: &'a ServerState,
        source_uri: &'a Uri,
    ) -> Vec<Location> {
        lsp.documents
            .iter()
            .filter_map(|doc| Some((UriExt::from_file_path(&doc.path)?, doc)))
            .flat_map(move |(uri, doc): (Uri, &Document)| {
                doc.links().filter_map(move |link| {
                    let target = link.target_str(&doc.source);
                    Self::resolve_and_check_target(lsp, doc, &target, source_uri)?;

                    let range = doc.source.slice(..).byte_to_lsp_range(link.span);
                    Some(Location::new(uri.clone(), range))
                })
            })
            .collect()
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

    pub(crate) fn is_source_reference(&self, uri: &gen_lsp_types::Uri, reference: Reference<'a>) -> bool {
        uri == self.source_uri && reference.span() == self.source_ref.span()
    }

    /// Check if a reference matches our source reference and return its location if so
    pub(crate) fn check_reference_match(
        &self,
        uri: &gen_lsp_types::Uri,
        doc: &'a Document,
        reference: Reference<'a>,
    ) -> Option<Location> {
        match self.source_ref {
            Reference::Header(header) => {
                let content = header.content_str(&self.source_doc.source);
                self.match_header_reference(uri, doc, reference, &content)
            }
            Reference::Link(link) => {
                let target = link.target_str(&self.source_doc.source);
                // Resolve the source link's target to compare with other references
                let resolved_target = match resolve_target_uri(self.lsp, self.source_doc, &target)
                {
                    Ok(target) => target,
                    Err(err) => {
                        tracing::error!("Source link target resolution failed: {:?}", err);
                        return None;
                    }
                };

                let header = link.header_str(&self.source_doc.source);
                self.match_link_reference(uri, doc, reference, header.as_deref(), &resolved_target)
            }
            Reference::FootnoteRef(footnote_ref) => {
                let identifier = footnote_ref.identifier_str(&self.source_doc.source);
                self.match_footnote_identifier(uri, doc, reference, &identifier)
            }
            Reference::FootnoteDef(footnote_def) => {
                let identifier = footnote_def.identifier_str(&self.source_doc.source);
                self.match_footnote_identifier(uri, doc, reference, &identifier)
            }
        }
    }

    /// Find other footnote references/definitions with the same identifier
    /// within the same document
    pub(crate) fn match_footnote_identifier(
        &self,
        uri: &gen_lsp_types::Uri,
        doc: &Document,
        reference: Reference<'a>,
        source_identifier: &str,
    ) -> Option<Location> {
        if doc.path != self.source_doc.path {
            return None;
        }

        match reference {
            Reference::FootnoteRef(footnote_ref) => {
                if footnote_ref.identifier_str(&doc.source) == source_identifier {
                    let range = doc.source.slice(..).byte_to_lsp_range(footnote_ref.span);
                    Some(Location::new(uri.clone(), range))
                } else {
                    None
                }
            }
            Reference::FootnoteDef(footnote_def) => {
                if footnote_def.identifier_str(&doc.source) == source_identifier {
                    let range = doc.source.slice(..).byte_to_lsp_range(footnote_def.span);
                    Some(Location::new(uri.clone(), range))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Find links that reference the given header
    pub(crate) fn match_header_reference(
        &self,
        uri: &gen_lsp_types::Uri,
        doc: &Document,
        reference: Reference<'a>,
        source_content: &str,
    ) -> Option<Location> {
        let Reference::Link(link) = reference else {
            return None;
        };

        let target = link.target_str(&doc.source);
        Self::resolve_and_check_target(self.lsp, self.source_doc, &target, self.source_uri)?;

        if let Some(link_header) = link.header_str(&doc.source)
            && normalized_headers_match(source_content, &link_header)
        {
            let range = doc.source.slice(..).byte_to_lsp_range(link.span);
            Some(Location::new(uri.clone(), range))
        } else {
            None
        }
    }

    pub(crate) fn match_link_reference(
        &self,
        uri: &gen_lsp_types::Uri,
        doc: &Document,
        reference: Reference<'a>,
        source_header: Option<&str>,
        source_target: &gen_lsp_types::Uri,
    ) -> Option<Location> {
        match reference {
            Reference::Link(link) => self
                .match_link_to_link(doc, link, source_header, source_target)
                .map(|_| {
                    let range = doc.source.slice(..).byte_to_lsp_range(link.span);
                    Location::new(uri.clone(), range)
                }),
            Reference::Header(header) => self
                .match_link_to_header(doc, header, uri, source_header, source_target)
                .map(|_| {
                    let range = doc.source.slice(..).byte_to_lsp_range(header.span);
                    Location::new(uri.clone(), range)
                }),
            Reference::FootnoteRef(_) | Reference::FootnoteDef(_) => None,
        }
    }

    pub(crate) fn match_link_to_link(
        &self,
        doc: &Document,
        link: &Link,
        source_header: Option<&str>,
        source_target: &gen_lsp_types::Uri,
    ) -> Option<()> {
        let target = link.target_str(&doc.source);
        Self::resolve_and_check_target(self.lsp, self.source_doc, &target, source_target)?;

        let reference_header = link.header_str(&doc.source);

        if source_header == reference_header.as_deref() {
            Some(())
        } else {
            None
        }
    }

    pub(crate) fn match_link_to_header(
        &self,
        doc: &Document,
        header: &Header,
        uri: &gen_lsp_types::Uri,
        source_header: Option<&str>,
        source_target: &gen_lsp_types::Uri,
    ) -> Option<()> {
        if uri != source_target {
            return None;
        }

        let header_content = header.content_str(&doc.source);
        let source_header = source_header?;

        if normalized_headers_match(&header_content, source_header) {
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
