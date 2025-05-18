pub mod references;

use lsp_types::{Position, Range, Uri};
use parser::{markdown_parser, InlineMarkdown, Markdown, Parser, Spanned};
use references::{LinkData, LinkHeader, Reference};
use ropey::Rope;
use std::str::FromStr;

#[derive(Debug)]
pub struct Document {
    pub uri: Uri,
    pub content: Rope,
    pub references: Vec<Reference>,
}

impl Document {
    pub fn new(uri: Uri, content: &str) -> Self {
        let mut s = Self {
            uri,
            content: Rope::from_str(content),
            references: Vec::new(),
        };
        s.parse_content();

        s
    }

    pub fn update(&mut self, new_content: &str) {
        self.content = Rope::from_str(new_content);
        self.parse_content();
    }

    pub fn find_reference_at_position(&self, position: Position) -> Option<&Reference> {
        let Position { line, character } = position;
        let text = self.content.slice(..);

        let line_str = text.line(line as usize);
        let character_byte_pos = line_str
            .chars()
            .take(character as usize)
            .map(|c| c.len_utf8())
            .sum::<usize>();

        let line_byte_idx = text.line_to_byte(line as usize);
        let cursor_byte_pos = line_byte_idx + character_byte_pos;

        self.references.iter().find(|reference| match reference {
            Reference::Link(data) | Reference::WikiLink(data) => {
                data.span.contains(&cursor_byte_pos)
            }
            Reference::Header { span, .. } => span.contains(&cursor_byte_pos),
            _ => false,
        })
    }

    fn parse_content(&mut self) {
        let input = self.content.slice(..).to_string();
        let (_frontmatter, body) =
            if let Some(parsed_markdown) = markdown_parser().parse(&input).into_output() {
                (parsed_markdown.frontmatter, parsed_markdown.body)
            } else {
                return;
            };

        self.references.clear();

        body.into_iter().for_each(|spanned| {
            let Spanned(markdown, span) = spanned;
            match markdown {
                Markdown::FootnoteDefinition { .. } => {}
                Markdown::Header { level, content } => {
                    let reference = Reference::Header {
                        level,
                        content: content.to_string(),
                        span: span.into_range(),
                    };
                    self.references.push(reference);
                }
                Markdown::Paragraph(inlines) => {
                    for inline in inlines {
                        let Spanned(inline_markdown, inline_span) = inline;

                        match inline_markdown {
                            InlineMarkdown::Link { title, uri, header } => {
                                let reference = create_link_reference(
                                    &self.uri,
                                    inline_span.into_range(),
                                    uri,
                                    Some(title),
                                    header,
                                );
                                self.references.push(reference);
                            }
                            InlineMarkdown::WikiLink {
                                target,
                                alias,
                                header,
                            } => {
                                let reference = create_link_reference(
                                    &self.uri,
                                    inline_span.into_range(),
                                    target,
                                    alias,
                                    header,
                                );
                                self.references.push(reference);
                            }
                            _ => {}
                        }
                    }
                }
                Markdown::Invalid => {}
            }
        });
    }

    pub fn span_to_range(&self, span: &std::ops::Range<usize>) -> Range {
        let start_line = self.content.byte_to_line(span.start);
        let end_line = self.content.byte_to_line(span.end);

        let line_start_char_idx = self.content.line_to_char(start_line);
        let line_end_char_idx = self.content.line_to_char(end_line);

        let start_char = self.content.byte_to_char(span.start) - line_start_char_idx;
        let end_char = self.content.byte_to_char(span.end) - line_end_char_idx;

        Range {
            start: Position::new(start_line as u32, start_char as u32),
            end: Position::new(end_line as u32, end_char as u32),
        }
    }
}

// Helper function to create a link reference
fn create_link_reference(
    source: &Uri,
    inline_span: std::ops::Range<usize>,
    target: &str,
    title: Option<&str>,
    header: Option<parser::LinkHeader>,
) -> Reference {
    Reference::Link(LinkData {
        source: source.clone(),
        span: inline_span,
        target: Uri::from_str(target).unwrap_or(Uri::from_str("").unwrap()),
        title: title.map(|t| t.to_string()),
        header: header.map(|parser::LinkHeader { level, content }| LinkHeader {
            level,
            content: content.to_string(),
        }),
    })
}
