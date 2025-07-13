use std::ops::Range;

use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Uri};
use miette::Result;
use parser::{markdown_parser, InlineMarkdownNode, LinkType, MarkdownNode, Parser, Spanned};
use references::{Reference, ReferenceKind};
use ropey::Rope;

pub mod references;

#[derive(Debug, Clone)]
pub struct Document {
    pub uri: Uri,
    pub version: i32,
    pub content: Rope,
    pub references: Vec<Reference>,
    pub diagnostics: Vec<Diagnostic>,
}

impl Document {
    pub fn new(uri: Uri, content: &str, version: i32) -> Result<Self> {
        let mut s = Self {
            uri,
            version,
            content: Rope::from_str(content),
            references: Vec::new(),
            diagnostics: Vec::new(),
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

        let input = self.content.slice(..).to_string();

        let (parsed_markdown, errors) = markdown_parser().parse(&input).into_output_errors();
        for err in errors {
            // TODO: Add to diagnostics
            self.diagnostics.push(Diagnostic {
                range: self.byte_to_lsp_range(&err.span().into_range()),
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
            return Ok(());
        };

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
                        range: self.byte_to_lsp_range(&span.into_range()),
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
                                        range: self.byte_to_lsp_range(&inline_span.into_range()),
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
                                        range: self.byte_to_lsp_range(&inline_span.into_range()),
                                    };
                                    self.references.push(reference);
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

    pub fn byte_to_lsp_range(&self, span: &Range<usize>) -> lsp_types::Range {
        let start_line = self.content.byte_to_line(span.start);
        let end_line = self.content.byte_to_line(span.end);

        let line_start_char_idx = self.content.line_to_char(start_line);
        let line_end_char_idx = self.content.line_to_char(end_line);

        let start_char = self.content.byte_to_char(span.start) - line_start_char_idx;
        let end_char = self.content.byte_to_char(span.end) - line_end_char_idx;

        lsp_types::Range::new(
            Position::new(start_line as u32, start_char as u32),
            Position::new(end_line as u32, end_char as u32),
        )
    }

    pub fn lsp_range_to_byte(&self, range: &lsp_types::Range) -> Range<usize> {
        let start_byte =
            self.content.line_to_byte(range.start.line as usize) + range.start.character as usize;
        let end_byte =
            self.content.line_to_byte(range.end.line as usize) + range.end.character as usize;

        start_byte..end_byte
    }
}
