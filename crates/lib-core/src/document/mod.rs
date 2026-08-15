use std::{collections::HashMap, fmt::Debug, path::PathBuf};

use gen_lsp_types::{Diagnostic, Position, Range as LspRange};
use lib_parser::new_markdown::{Block, BlockKind, Inline, InlineKind, Parser, Span};
use miette::Result;
use references::{ReferenceKindOld, ReferenceOld};
use ropey::{Rope, RopeSlice};

use crate::document::{
    index::{Header, Link, LinkKind},
    metadata::FrontmatterValue,
};

pub mod index;
pub mod metadata;
pub mod references;

pub enum Reference<'a> {
    Link(&'a Link),
    Header(&'a Header),
}

impl<'a> Reference<'a> {
    pub fn span(&self) -> Span {
        match self {
            Reference::Link(link) => link.span,
            Reference::Header(header) => header.span,
        }
    }
}

pub struct Edit {
    span: Span,
    new_text: String,
}

impl Edit {
    pub fn new(span: Span, new_text: String) -> Self {
        Self { span, new_text }
    }

    pub fn delta(&self) -> isize {
        self.new_text.len() as isize - self.span.len() as isize
    }
}

#[derive(Default, Debug, Clone)]
pub struct Document {
    pub path: PathBuf,
    pub version: i32,
    pub source: Rope,

    headers: Vec<Header>,
    links: Vec<Link>,

    // TODO: Delete
    pub frontmatter: HashMap<String, FrontmatterValue>,
    pub references: Vec<ReferenceOld>,
    pub diagnostics: Vec<Diagnostic>,

    // TODO: remove from here as only the lsp server cares about this
    pub is_open: bool,
}

impl Document {
    pub fn new(path: PathBuf, content: &str, version: i32) -> Result<Self> {
        let mut document = Self {
            path,
            version,
            source: Rope::from_str(content),
            ..Default::default()
        };
        document.reparse(content);
        Ok(document)
    }

    pub fn update(&mut self, content: &str, version: i32) -> Result<()> {
        self.source = Rope::from_str(content);
        self.version = version;
        self.reparse(content);
        Ok(())
    }

    pub fn get_reference_at_position_old(&self, position: Position) -> Option<&ReferenceOld> {
        self.references
            .iter()
            .find(|reference| reference.contains_position(position))
    }

    pub fn links(&self) -> impl Iterator<Item = &Link> {
        self.links.iter()
    }

    pub fn headers(&self) -> impl Iterator<Item = &Header> {
        self.headers.iter()
    }

    pub fn get_reference_at_offset<'a>(&'a self, byte_offset: usize) -> Option<Reference<'a>> {
        self.links()
            .map(Reference::Link)
            .chain(self.headers().map(Reference::Header))
            .find(|r| r.span().contains_offset(byte_offset))
    }

    pub fn apply_edit(&mut self, edit: Edit) {
        todo!()
    }

    /// will parse entire content and reset document state
    fn reparse(&mut self, content: &str) {
        let blocks = Parser::new(content).parse();

        self.headers.clear();
        self.links.clear();

        for block in &blocks {
            extract_block(self, block, content);
        }

        self.references = build_references(self, content);
    }
}

fn byte_offset_to_position(slice: &RopeSlice, byte_offset: usize) -> Position {
    let line_idx = slice.byte_to_line(byte_offset);
    let line_start_char = slice.line_to_char(line_idx);
    let global_char_idx = slice.byte_to_char(byte_offset);
    let char_offset = global_char_idx - line_start_char;
    Position::new(line_idx as u32, char_offset as u32)
}

fn byte_span_to_lsp_range(slice: &RopeSlice, span: Span) -> LspRange {
    let start_pos = byte_offset_to_position(slice, span.start);
    let end_pos = byte_offset_to_position(slice, span.end);
    LspRange::new(start_pos, end_pos)
}

fn build_references(doc: &Document, content: &str) -> Vec<ReferenceOld> {
    let slice = doc.source.slice(..);
    let mut references = Vec::with_capacity(doc.headers.len() + doc.links.len());

    for header in &doc.headers {
        references.push(ReferenceOld {
            kind: ReferenceKindOld::Header {
                level: header.level as usize,
                content: header.text_span.as_str(content).to_string(),
            },
            range: byte_span_to_lsp_range(&slice, header.span),
        });
    }

    for link in &doc.links {
        let header = link.header_str(&doc.source).map(|h| h.to_string());
        let kind = match &link.kind {
            LinkKind::Wiki { target, alias, .. } => ReferenceKindOld::WikiLink {
                target: target.as_str(content).to_string(),
                alias: alias.map(|a| a.as_str(content).to_string()),
                header,
            },
            LinkKind::Inline {
                label, url, title, ..
            } => ReferenceKindOld::Link {
                target: url.as_str(content).to_string(),
                alt_text: label.as_str(content).to_string(),
                title: title.map(|t| t.as_str(content).to_string()),
                header,
            },
        };
        references.push(ReferenceOld {
            kind,
            range: byte_span_to_lsp_range(&slice, link.span),
        });
    }

    references
}

fn extract_block(doc: &mut Document, block: &Block, content: &str) {
    match &block.kind {
        BlockKind::Heading { level, children } => {
            let text_span = span_of_children(children);
            doc.headers.push(Header {
                span: block.span,
                text_span,
                level: *level,
            });
            extract_inlines(doc, children, content);
        }
        BlockKind::Paragraph { children } => {
            extract_inlines(doc, children, content);
        }
        BlockKind::List { children, .. } => {
            for child in children {
                extract_block(doc, child, content);
            }
        }
    }
}

/// Splits a span on `#`, returning `(before, Some(after))` or `(original, None)`.
fn split_span_on_hash(span: Span, content: &str) -> (Span, Option<Span>) {
    let text = span.as_str(content);
    match text.find('#') {
        Some(offset) => {
            let target = Span::new(span.start, span.start + offset);
            let header = Span::new(span.start + offset + 1, span.end);
            (target, Some(header))
        }
        None => (span, None),
    }
}

fn extract_inlines(doc: &mut Document, inlines: &[Inline], content: &str) {
    for inline in inlines {
        match &inline.kind {
            InlineKind::Wikilink {
                target_span,
                children,
            } => {
                let (target, header) = split_span_on_hash(*target_span, content);
                doc.links.push(Link {
                    span: inline.span,
                    kind: LinkKind::Wiki {
                        target,
                        header,
                        alias: children.as_deref().map(span_of_children),
                    },
                });
            }
            InlineKind::Link { children, url_span } => {
                let (url, header) = split_span_on_hash(*url_span, content);
                doc.links.push(Link {
                    span: inline.span,
                    kind: LinkKind::Inline {
                        label: span_of_children(children),
                        url,
                        header,
                        title: None,
                    },
                });
            }
            InlineKind::Bold { children }
            | InlineKind::Italic { children }
            | InlineKind::BoldItalic { children }
            | InlineKind::Strikethrough { children } => {
                extract_inlines(doc, children, content);
            }
            InlineKind::Text | InlineKind::Footnote { .. } => {}
        }
    }
}

fn span_of_children(children: &[Inline]) -> Span {
    match (children.first(), children.last()) {
        (Some(first), Some(last)) => Span::new(first.span.start, last.span.end),
        _ => Span::new(0, 0),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn doc(content: &str) -> Document {
        Document::new(PathBuf::from("test.md"), content, 0).unwrap()
    }

    #[test]
    fn builds_references_for_headers_and_links() {
        let d = doc("# Hello\n\n[[note#Section|alias]] and [text](url#Frag)");
        assert_eq!(d.references.len(), 3);

        let ReferenceKindOld::Header { level, content } = &d.references[0].kind else {
            panic!("expected header reference");
        };
        assert_eq!(*level, 1);
        assert_eq!(content, "Hello");

        let ReferenceKindOld::WikiLink {
            target,
            alias,
            header,
        } = &d.references[1].kind
        else {
            panic!("expected wikilink reference");
        };
        assert_eq!(target, "note");
        assert_eq!(alias.as_deref(), Some("alias"));
        assert_eq!(header.as_deref(), Some("Section"));

        let ReferenceKindOld::Link {
            target,
            alt_text,
            header,
            ..
        } = &d.references[2].kind
        else {
            panic!("expected link reference");
        };
        assert_eq!(target, "url");
        assert_eq!(alt_text, "text");
        assert_eq!(header.as_deref(), Some("Frag"));
    }

    #[test]
    fn extracts_h1() {
        let d = doc("# Hello");
        assert_eq!(d.headers.len(), 1);
        assert_eq!(d.headers[0].level, 1);
        assert_eq!(d.headers[0].text_span.as_str("# Hello"), "Hello");
    }

    #[test]
    fn extracts_multiple_headers() {
        let d = doc("# One\n## Two\n### Three");
        assert_eq!(d.headers.len(), 3);
        assert_eq!(d.headers[0].level, 1);
        assert_eq!(d.headers[1].level, 2);
        assert_eq!(d.headers[2].level, 3);
    }

    #[test]
    fn extracts_wikilink() {
        let d = doc("[[target]]");
        assert_eq!(d.links.len(), 1);
        let LinkKind::Wiki { target, alias, .. } = &d.links[0].kind else {
            panic!("expected wiki link");
        };
        assert_eq!(target.as_str("[[target]]"), "target");
        assert!(alias.is_none());
    }

    #[test]
    fn extracts_wikilink_with_alias() {
        let d = doc("[[target|my alias]]");
        let LinkKind::Wiki { target, alias, .. } = &d.links[0].kind else {
            panic!("expected wiki link");
        };
        assert_eq!(target.as_str("[[target|my alias]]"), "target");
        assert_eq!(alias.unwrap().as_str("[[target|my alias]]"), "my alias");
    }

    #[test]
    fn extracts_inline_link() {
        let content = "[label](https://example.com)";
        let d = doc(content);
        let LinkKind::Inline { label, url, .. } = &d.links[0].kind else {
            panic!("expected inline link");
        };
        assert_eq!(label.as_str(content), "label");
        assert_eq!(url.as_str(content), "https://example.com");
    }

    #[test]
    fn extracts_link_in_heading() {
        let d = doc("# See [Google](https://google.com)");
        assert_eq!(d.headers.len(), 1);
        assert_eq!(d.links.len(), 1);
    }

    #[test]
    fn extracts_headers_and_links_together() {
        let d = doc("# Heading\n\nSome text with [[a]] and [b](https://b.com)");
        assert_eq!(d.headers.len(), 1);
        assert_eq!(d.links.len(), 2);
    }

    #[test]
    fn empty_document() {
        let d = doc("");
        assert!(d.headers.is_empty());
        assert!(d.links.is_empty());
    }
}
