use std::{collections::HashMap, fmt::Debug, path::PathBuf};

use gen_lsp_types::{Diagnostic, Position};
use lib_parser::new_markdown::{Block, BlockKind, Inline, InlineKind, Parser, Span};
use miette::Result;
use references::Reference;
use ropey::Rope;

use crate::document::{
    index::{Header, Link, LinkKind},
    metadata::FrontmatterValue,
};

pub mod index;
pub mod metadata;
pub mod references;

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

    pub headers: Vec<Header>,
    pub links: Vec<Link>,

    // TODO: Delete
    pub frontmatter: HashMap<String, FrontmatterValue>,
    pub references: Vec<Reference>,
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

    pub fn get_reference_at_position(&self, position: Position) -> Option<&Reference> {
        self.references
            .iter()
            .find(|reference| reference.contains_position(position))
    }

    // fn parse_and_analyze(&mut self) -> Result<()> {
    //     self.references.clear();
    //     self.diagnostics.clear();
    //
    //     let doc_content_slice = self.source.slice(..);
    //     let input = doc_content_slice.to_string();
    //
    //     let (parsed_markdown, errors) = markdown_parser().parse(&input).into_output_errors();
    //     for err in errors {
    //         self.diagnostics.push(Diagnostic {
    //             range: doc_content_slice.byte_to_lsp_range(&err.span().into_range()),
    //             severity: Some(DiagnosticSeverity::Warning),
    //             code: None,
    //             code_description: None,
    //             source: Some("parser".to_string()),
    //             message: err.reason().to_string(),
    //             related_information: None,
    //             tags: None,
    //             data: None,
    //         });
    //     }
    //
    //     let Some(parsed_markdown) = parsed_markdown else {
    //         tracing::debug!("Failed to parse");
    //         return Ok(());
    //     };
    //
    //     let frontmatter = parsed_markdown.frontmatter;
    //     if let Some(frontmatter) = frontmatter {
    //         for (key, val) in frontmatter.0 {
    //             self.frontmatter
    //                 .insert(key.to_string(), FrontmatterValue::from(val));
    //         }
    //     }
    //
    //     let body = parsed_markdown.body;
    //     body.into_iter().for_each(|spanned| {
    //         let Spanned(markdown, span) = spanned;
    //         match markdown {
    //             MarkdownNode::Header { level, content } => {
    //                 let reference = Reference {
    //                     kind: ReferenceKind::Header {
    //                         level,
    //                         content: content.to_string(),
    //                     },
    //                     range: doc_content_slice.byte_to_lsp_range(&span.into_range()),
    //                 };
    //                 self.references.push(reference);
    //             }
    //             MarkdownNode::Paragraph(inlines) => {
    //                 for inline in inlines {
    //                     let Spanned(inline_markdown, inline_span) = inline;
    //
    //                     if let InlineMarkdownNode::Link(link) = inline_markdown {
    //                         match link {
    //                             LinkType::InlineLink { text, uri, header } => {
    //                                 let reference = Reference {
    //                                     kind: ReferenceKind::Link {
    //                                         target: uri.to_string(),
    //                                         alt_text: text.to_string(),
    //                                         title: None,
    //                                         header: header.map(|x| x.to_string()),
    //                                     },
    //                                     range: doc_content_slice
    //                                         .byte_to_lsp_range(&inline_span.into_range()),
    //                                 };
    //                                 self.references.push(reference);
    //                             }
    //                             LinkType::WikiLink {
    //                                 target,
    //                                 display_text,
    //                                 header,
    //                             } => {
    //                                 let reference = Reference {
    //                                     kind: ReferenceKind::WikiLink {
    //                                         target: target.to_string(),
    //                                         alias: display_text.map(|d| d.to_string()),
    //                                         header: header.map(|x| x.to_string()),
    //                                     },
    //                                     range: doc_content_slice
    //                                         .byte_to_lsp_range(&inline_span.into_range()),
    //                                 };
    //                                 self.references.push(reference);
    //                             }
    //                             LinkType::ImageLink { .. } => {
    //                                 tracing::debug!("Not currently supporting images")
    //                             }
    //                         }
    //                     }
    //                 }
    //             }
    //             MarkdownNode::ListItem {
    //                 checkbox: _,
    //                 content: list_content,
    //             } => {
    //                 // Process links inside list item content (same as paragraph)
    //                 for inline in list_content {
    //                     let Spanned(inline_markdown, inline_span) = inline;
    //
    //                     if let InlineMarkdownNode::Link(link) = inline_markdown {
    //                         match link {
    //                             LinkType::InlineLink { text, uri, header } => {
    //                                 let reference = Reference {
    //                                     kind: ReferenceKind::Link {
    //                                         target: uri.to_string(),
    //                                         alt_text: text.to_string(),
    //                                         title: None,
    //                                         header: header.map(|x| x.to_string()),
    //                                     },
    //                                     range: doc_content_slice
    //                                         .byte_to_lsp_range(&inline_span.into_range()),
    //                                 };
    //                                 self.references.push(reference);
    //                             }
    //                             LinkType::WikiLink {
    //                                 target,
    //                                 display_text,
    //                                 header,
    //                             } => {
    //                                 let reference = Reference {
    //                                     kind: ReferenceKind::WikiLink {
    //                                         target: target.to_string(),
    //                                         alias: display_text.map(|d| d.to_string()),
    //                                         header: header.map(|x| x.to_string()),
    //                                     },
    //                                     range: doc_content_slice
    //                                         .byte_to_lsp_range(&inline_span.into_range()),
    //                                 };
    //                                 self.references.push(reference);
    //                             }
    //                             LinkType::ImageLink { .. } => {
    //                                 tracing::debug!("Not currently supporting images")
    //                             }
    //                         }
    //                     }
    //                 }
    //             }
    //             _ => {}
    //         }
    //     });
    //
    //     Ok(())
    // }

    pub fn apply_edit(&mut self, edit: Edit) {
        todo!()
    }

    /// will parse entire content and reset document state
    fn reparse(&mut self, content: &str) {
        let blocks = Parser::new(content).parse();

        self.headers.clear();
        self.links.clear();

        for block in &blocks {
            extract_block(self, block);
        }
    }
}

fn extract_block(doc: &mut Document, block: &Block) {
    match &block.kind {
        BlockKind::Heading { level, children } => {
            let text_span = span_of_children(children);
            doc.headers.push(Header {
                span: block.span,
                text_span,
                level: *level,
            });
            extract_inlines(doc, children);
        }
        BlockKind::Paragraph { children } => {
            extract_inlines(doc, children);
        }
        BlockKind::List { children, .. } => {
            for child in children {
                extract_block(doc, child);
            }
        }
    }
}

fn extract_inlines(doc: &mut Document, inlines: &[Inline]) {
    for inline in inlines {
        match &inline.kind {
            InlineKind::Wikilink {
                target_span,
                children,
            } => {
                doc.links.push(Link {
                    span: inline.span,
                    kind: LinkKind::Wiki {
                        target: *target_span,
                        alias: children.as_deref().map(span_of_children),
                    },
                });
            }
            InlineKind::Link { children, url_span } => {
                doc.links.push(Link {
                    span: inline.span,
                    kind: LinkKind::Inline {
                        label: span_of_children(children),
                        url: *url_span,
                        title: None,
                    },
                });
            }
            InlineKind::Bold { children }
            | InlineKind::Italic { children }
            | InlineKind::BoldItalic { children }
            | InlineKind::Strikethrough { children } => {
                extract_inlines(doc, children);
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
        let LinkKind::Wiki { target, alias } = &d.links[0].kind else {
            panic!("expected wiki link");
        };
        assert_eq!(target.as_str("[[target]]"), "target");
        assert!(alias.is_none());
    }

    #[test]
    fn extracts_wikilink_with_alias() {
        let d = doc("[[target|my alias]]");
        let LinkKind::Wiki { target, alias } = &d.links[0].kind else {
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
