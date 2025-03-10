pub mod references;
pub mod uri;

use parser::{markdown_parser, InlineMarkdown, Markdown, Parser, Spanned};
use references::{LinkData, LinkHeader, Reference};
use ropey::Rope;
use uri::URI;

use crate::lsp::{Position, Range};

pub type DocumentUri = URI;

#[derive(Debug)]
pub struct Document {
    pub uri: URI,
    pub content: Rope,
    pub references: Vec<Reference>,
}

impl Document {
    pub fn new(uri: URI, content: &str) -> Self {
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

    pub fn find_reference_at_position(&self, position: Position) -> Option<&LinkData> {
        let Position { line, character } = position;
        let text = self.content.slice(..);

        let line_str = text.line(line);
        let character_byte_pos = line_str
            .chars()
            .take(character)
            .map(|c| c.len_utf8())
            .sum::<usize>();

        let line_byte_idx = text.line_to_byte(line);
        let cursor_byte_pos = line_byte_idx + character_byte_pos;

        self.references
            .iter()
            .find_map(|reference| match reference {
                Reference::Link(data) | Reference::WikiLink(data)
                    if data.span.contains(&cursor_byte_pos) =>
                {
                    Some(data)
                }
                _ => None,
            })
    }

    fn parse_content(&mut self) {
        let input = self.content.slice(..).to_string();
        let markdown_spans =
            if let Some(markdown_tokens) = markdown_parser().parse(&input).into_output() {
                markdown_tokens
            } else {
                return;
            };

        self.references.clear();

        markdown_spans.into_iter().for_each(|spanned| {
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

                        if let InlineMarkdown::Link { title, url, header } = inline_markdown {
                            let link_data = LinkData {
                                source: self.uri.clone(),
                                span: inline_span.into_range(),
                                target: URI(url.to_string()),
                                title: Some(title.to_string()),
                                header: header.map(|h| LinkHeader {
                                    level: 1,
                                    content: h.to_string(),
                                }),
                            };
                            let reference = Reference::Link(link_data);
                            self.references.push(reference);
                        }

                        if let InlineMarkdown::WikiLink {
                            target,
                            alias,
                            header,
                        } = inline_markdown
                        {
                            let link_data = LinkData {
                                source: self.uri.clone(),
                                span: inline_span.into_range(),
                                target: URI(target.to_string()),
                                title: alias.map(String::from),
                                header: header.map(|parser::LinkHeader { level, content }| {
                                    LinkHeader {
                                        level,
                                        content: content.to_string(),
                                    }
                                }),
                            };
                            let reference = Reference::Link(link_data);
                            self.references.push(reference);
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
            start: Position::new(start_line, start_char),
            end: Position::new(end_line, end_char),
        }
    }
}
