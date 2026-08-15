use std::{collections::HashMap, fmt::Debug, path::PathBuf};

use lib_parser::markdown::{Block, BlockKind, Inline, InlineKind, Parser, Span};
use miette::Result;
use ropey::Rope;

use crate::document::{
    index::{
        DiagnosticCode, FootnoteDefinition, FootnoteReference, Header, Link, LinkKind, Severity,
        Tag,
    },
    metadata::FrontmatterValue,
};

pub mod index;
pub mod metadata;

#[derive(Clone, Copy)]
pub enum Reference<'a> {
    Link(&'a Link),
    Header(&'a Header),
    FootnoteRef(&'a FootnoteReference),
    FootnoteDef(&'a FootnoteDefinition),
}

impl<'a> Reference<'a> {
    pub fn span(&self) -> Span {
        match self {
            Reference::Link(link) => link.span,
            Reference::Header(header) => header.span,
            Reference::FootnoteRef(footnote) => footnote.span,
            Reference::FootnoteDef(footnote) => footnote.span,
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
    footnote_definitions: Vec<FootnoteDefinition>,
    footnote_references: Vec<FootnoteReference>,
    tags: Vec<Tag>,
    diagnostics: Vec<index::Diagnostic>,

    pub frontmatter: HashMap<String, FrontmatterValue>,
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

    pub fn links(&self) -> impl Iterator<Item = &Link> {
        self.links.iter()
    }

    pub fn headers(&self) -> impl Iterator<Item = &Header> {
        self.headers.iter()
    }

    pub fn footnote_definitions(&self) -> impl Iterator<Item = &FootnoteDefinition> {
        self.footnote_definitions.iter()
    }

    pub fn footnote_references(&self) -> impl Iterator<Item = &FootnoteReference> {
        self.footnote_references.iter()
    }

    pub fn find_footnote_definition(&self, identifier: &str) -> Option<&FootnoteDefinition> {
        self.footnote_definitions()
            .find(|def| def.identifier_str(&self.source) == identifier)
    }

    pub fn tags(&self) -> impl Iterator<Item = &Tag> {
        self.tags.iter()
    }

    pub fn diagnostics(&self) -> impl Iterator<Item = &index::Diagnostic> {
        self.diagnostics.iter()
    }

    pub fn get_reference_at_offset<'a>(&'a self, byte_offset: usize) -> Option<Reference<'a>> {
        self.links()
            .map(Reference::Link)
            .chain(self.headers().map(Reference::Header))
            .chain(self.footnote_references().map(Reference::FootnoteRef))
            .chain(self.footnote_definitions().map(Reference::FootnoteDef))
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
        self.footnote_definitions.clear();
        self.footnote_references.clear();
        self.tags.clear();
        self.diagnostics.clear();
        self.frontmatter.clear();

        for block in &blocks {
            extract_block(self, block, content);
        }
    }
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
        BlockKind::ListItem {
            inline, children, ..
        } => {
            extract_inlines(doc, inline, content);
            for child in children {
                extract_block(doc, child, content);
            }
        }
        BlockKind::FootnoteDefinition {
            identifier,
            children,
        } => {
            doc.footnote_definitions.push(FootnoteDefinition {
                span: block.span,
                identifier: *identifier,
                content_span: span_of_children(children),
            });
            extract_inlines(doc, children, content);
        }
        BlockKind::Frontmatter => {
            let text = block.span.as_str(content);
            if let Some(frontmatter) = lib_parser::yaml::parse_frontmatter(text) {
                for (key, value) in frontmatter.0 {
                    doc.frontmatter
                        .insert(key.to_string(), FrontmatterValue::from(value));
                }
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
            InlineKind::Image { alt_span, url_span } => {
                doc.links.push(Link {
                    span: inline.span,
                    kind: LinkKind::Image {
                        alt: *alt_span,
                        url: *url_span,
                    },
                });
            }
            InlineKind::Tag { name } => {
                let name_text = name.as_str(content);
                if name_text.ends_with('/') || name_text.ends_with('-') {
                    doc.diagnostics.push(index::Diagnostic {
                        span: inline.span,
                        severity: Severity::Warning,
                        code: DiagnosticCode::MalformedTag,
                        message: format!(
                            "Tag '#{name_text}' ends with a dangling separator"
                        ),
                    });
                }

                doc.tags.push(Tag {
                    span: inline.span,
                    name_span: *name,
                });
            }
            InlineKind::Bold { children }
            | InlineKind::Italic { children }
            | InlineKind::BoldItalic { children }
            | InlineKind::Strikethrough { children } => {
                extract_inlines(doc, children, content);
            }
            InlineKind::Footnote { identifier } => {
                doc.footnote_references.push(FootnoteReference {
                    span: inline.span,
                    identifier: *identifier,
                });
            }
            InlineKind::Text => {}
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
        let content = "# Hello\n\n[[note#Section|alias]] and [text](url#Frag)";
        let d = doc(content);

        assert_eq!(d.headers().count(), 1);
        assert_eq!(d.links().count(), 2);

        let header = d.headers().next().unwrap();
        assert_eq!(header.level, 1);
        assert_eq!(header.content_str(&d.source), "Hello");

        let mut links = d.links();

        let LinkKind::Wiki {
            target,
            header,
            alias,
        } = &links.next().unwrap().kind
        else {
            panic!("expected wikilink reference");
        };
        assert_eq!(target.as_str(content), "note");
        assert_eq!(header.map(|h| h.as_str(content)), Some("Section"));
        assert_eq!(alias.map(|a| a.as_str(content)), Some("alias"));

        let LinkKind::Inline {
            label,
            url,
            header,
            ..
        } = &links.next().unwrap().kind
        else {
            panic!("expected link reference");
        };
        assert_eq!(url.as_str(content), "url");
        assert_eq!(label.as_str(content), "text");
        assert_eq!(header.map(|h| h.as_str(content)), Some("Frag"));
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
    fn extracts_frontmatter() {
        let d = doc("---\ntitle: Hello\ntags:\n  - a\n  - b\n---\n# Heading");
        assert_eq!(
            d.frontmatter.get("title").and_then(|v| v.as_string()),
            Some("Hello")
        );
        assert_eq!(
            d.frontmatter.get("tags").and_then(|v| v.as_list()),
            Some(&["a".to_string(), "b".to_string()][..])
        );
        assert_eq!(d.headers.len(), 1);
    }

    #[test]
    fn extracts_tag() {
        let content = "hello #project/sub world";
        let d = doc(content);

        assert_eq!(d.tags().count(), 1);
        let tag = d.tags().next().unwrap();
        assert_eq!(tag.name_span.as_str(content), "project/sub");
    }

    #[test]
    fn extracts_image() {
        let content = "![alt text](image.png)";
        let d = doc(content);

        assert_eq!(d.links.len(), 1);
        let LinkKind::Image { alt, url } = &d.links[0].kind else {
            panic!("expected image link");
        };
        assert_eq!(alt.as_str(content), "alt text");
        assert_eq!(url.as_str(content), "image.png");
    }

    #[test]
    fn extracts_footnote_definition() {
        let content = "See[^note] below\n\n[^note]: the note text";
        let d = doc(content);

        assert_eq!(d.footnote_definitions().count(), 1);
        let def = d.footnote_definitions().next().unwrap();
        assert_eq!(def.identifier.as_str(content), "note");
        assert_eq!(def.content_span.as_str(content), "the note text");
    }

    #[test]
    fn empty_document() {
        let d = doc("");
        assert!(d.headers.is_empty());
        assert!(d.links.is_empty());
    }
}
