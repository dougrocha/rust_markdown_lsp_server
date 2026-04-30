use std::{collections::HashMap, fmt::Debug};

use lib_parser::{InlineMarkdownNode, LinkType, MarkdownNode, Parser, Spanned, markdown_parser};
use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Uri};
use miette::Result;
use references::{Reference, ReferenceKind};
use ropey::Rope;

use crate::{document::metadata::FrontmatterValue, text_buffer_conversions::TextBufferConversions};

pub mod metadata;
pub mod references;

#[derive(Debug, Clone)]
pub struct Document {
    pub uri: Uri,
    pub frontmatter: HashMap<String, FrontmatterValue>,
    pub version: i32,
    pub content: Rope,
    pub references: Vec<Reference>,
    pub diagnostics: Vec<Diagnostic>,
    pub is_open: bool,
}

impl Document {
    pub fn new(uri: Uri, content: &str, version: i32) -> Result<Self> {
        let mut s = Self {
            uri,
            version,
            content: Rope::from_str(content),
            references: Vec::new(),
            diagnostics: Vec::new(),
            is_open: false,
            frontmatter: HashMap::new(),
        };
        s.parse_and_analyze()?;

        Ok(s)
    }

    pub fn update(&mut self, content: &str, version: i32) -> Result<()> {
        self.content = Rope::from_str(content);
        self.version = version;
        self.parse_and_analyze()?;
        Ok(())
    }

    pub fn get_reference_at_position(&self, position: Position) -> Option<&Reference> {
        self.references
            .iter()
            .find(|reference| reference.contains_position(position))
    }

    fn parse_and_analyze(&mut self) -> Result<()> {
        self.references.clear();
        self.diagnostics.clear();

        let doc_content_slice = self.content.slice(..);
        let input = doc_content_slice.to_string();

        let (parsed_markdown, errors) = markdown_parser().parse(&input).into_output_errors();
        for err in errors {
            self.diagnostics.push(Diagnostic {
                range: doc_content_slice.byte_to_lsp_range(&err.span().into_range()),
                severity: Some(DiagnosticSeverity::WARNING),
                code: None,
                code_description: None,
                source: Some("parser".to_string()),
                message: err.reason().to_string(),
                related_information: None,
                tags: None,
                data: None,
            });
        }

        let Some(parsed_markdown) = parsed_markdown else {
            tracing::debug!("Failed to parse");
            return Ok(());
        };

        let frontmatter = parsed_markdown.frontmatter;
        if let Some(frontmatter) = frontmatter {
            for (key, val) in frontmatter.0 {
                self.frontmatter
                    .insert(key.to_string(), FrontmatterValue::from(val));
            }
        }

        let body = parsed_markdown.body;
        body.into_iter().for_each(|spanned| {
            let Spanned(markdown, span) = spanned;
            match markdown {
                MarkdownNode::Header { level, content } => {
                    let reference = Reference {
                        kind: ReferenceKind::Header {
                            level,
                            content: content.to_string(),
                        },
                        range: doc_content_slice.byte_to_lsp_range(&span.into_range()),
                    };
                    self.references.push(reference);
                }
                MarkdownNode::Paragraph(inlines) => {
                    for inline in inlines {
                        let Spanned(inline_markdown, inline_span) = inline;

                        if let InlineMarkdownNode::Link(link) = inline_markdown {
                            match link {
                                LinkType::InlineLink { text, uri, header } => {
                                    let reference = Reference {
                                        kind: ReferenceKind::Link {
                                            target: uri.to_string(),
                                            alt_text: text.to_string(),
                                            title: None,
                                            header: header.map(|x| x.to_string()),
                                        },
                                        range: doc_content_slice
                                            .byte_to_lsp_range(&inline_span.into_range()),
                                    };
                                    self.references.push(reference);
                                }
                                LinkType::WikiLink {
                                    target,
                                    display_text,
                                    header,
                                } => {
                                    let reference = Reference {
                                        kind: ReferenceKind::WikiLink {
                                            target: target.to_string(),
                                            alias: display_text.map(|d| d.to_string()),
                                            header: header.map(|x| x.to_string()),
                                        },
                                        range: doc_content_slice
                                            .byte_to_lsp_range(&inline_span.into_range()),
                                    };
                                    self.references.push(reference);
                                }
                                LinkType::ImageLink { .. } => {
                                    tracing::debug!("Not currently supporting images")
                                }
                            }
                        }
                    }
                }
                MarkdownNode::ListItem {
                    checkbox: _,
                    content: list_content,
                } => {
                    // Process links inside list item content (same as paragraph)
                    for inline in list_content {
                        let Spanned(inline_markdown, inline_span) = inline;

                        if let InlineMarkdownNode::Link(link) = inline_markdown {
                            match link {
                                LinkType::InlineLink { text, uri, header } => {
                                    let reference = Reference {
                                        kind: ReferenceKind::Link {
                                            target: uri.to_string(),
                                            alt_text: text.to_string(),
                                            title: None,
                                            header: header.map(|x| x.to_string()),
                                        },
                                        range: doc_content_slice
                                            .byte_to_lsp_range(&inline_span.into_range()),
                                    };
                                    self.references.push(reference);
                                }
                                LinkType::WikiLink {
                                    target,
                                    display_text,
                                    header,
                                } => {
                                    let reference = Reference {
                                        kind: ReferenceKind::WikiLink {
                                            target: target.to_string(),
                                            alias: display_text.map(|d| d.to_string()),
                                            header: header.map(|x| x.to_string()),
                                        },
                                        range: doc_content_slice
                                            .byte_to_lsp_range(&inline_span.into_range()),
                                    };
                                    self.references.push(reference);
                                }
                                LinkType::ImageLink { .. } => {
                                    tracing::debug!("Not currently supporting images")
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        });

        Ok(())
    }
}
