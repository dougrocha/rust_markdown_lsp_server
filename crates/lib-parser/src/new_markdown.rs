#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn as_str<'src>(&self, src: &'src str) -> &'src str {
        &src[self.start..self.end]
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug, Clone, PartialEq)]
enum BlockKind {
    Heading { level: u8, children: Vec<Inline> },
    Paragraph { children: Vec<Inline> },
    Error,
}

#[derive(Debug, Clone, PartialEq)]
struct Block {
    kind: BlockKind,
    span: Span,
}

#[derive(Debug, Clone, PartialEq)]
enum InlineKind {
    Text,
    Bold {
        children: Vec<Inline>,
    },
    Link {
        children: Vec<Inline>,
        url_span: Span,
    },
    Wikilink {
        target_span: Span,
        children: Option<Vec<Inline>>,
    },
}

#[derive(Debug, Clone, PartialEq)]
struct Inline {
    kind: InlineKind,
    span: Span,
}

#[derive(Debug, PartialEq)]
pub struct Document {
    blocks: Vec<Block>,
}

#[derive(Debug)]
struct Cursor<'src> {
    src: &'src str,
    pos: usize,
}

impl<'src> Cursor<'src> {
    fn new(source: &'src str) -> Self {
        Self {
            src: source,
            pos: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.src.len()
    }

    fn next(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn consume<F>(&mut self, f: F) -> bool
    where
        F: Fn(char) -> bool,
    {
        if self.peek().is_some_and(&f) {
            self.next();
            true
        } else {
            false
        }
    }

    fn consume_while<F>(&mut self, f: F) -> Span
    where
        F: Fn(char) -> bool,
    {
        let start = self.pos;
        while self.peek().is_some_and(&f) {
            self.next();
        }
        Span::new(start, self.pos)
    }
}

pub struct Parser<'src> {
    cursor: Cursor<'src>,
}

impl<'src> Parser<'src> {
    pub fn new(source: &'src str) -> Parser<'src> {
        Parser {
            cursor: Cursor::new(source),
        }
    }

    pub fn parse(mut self) -> Document {
        let mut blocks = Vec::new();

        while !self.cursor.is_eof() {
            if self.cursor.consume(|c| c == '\n') {
                continue;
            }

            blocks.push(self.parse_block());
        }

        Document { blocks }
    }

    fn parse_block(&mut self) -> Block {
        match self.cursor.peek() {
            Some('#') => self.parse_heading(),
            Some(_) => self.parse_paragraph(),
            None => unreachable!("eof?"),
        }
    }

    fn parse_heading(&mut self) -> Block {
        let start = self.cursor.pos;

        let level = self.cursor.consume_while(|c| c == '#').len();
        let spaces = self.cursor.consume_while(|c| c == ' ').len();
        if spaces == 0 {
            self.cursor.pos = start;
            return self.parse_paragraph();
        }

        let inline_span = Span::new(
            self.cursor.pos,
            self.cursor.consume_while(|c| c != '\n').end,
        );
        let kind = BlockKind::Heading {
            level: level as u8,
            children: self.parse_inline(inline_span),
        };

        self.cursor.consume(|c| c == '\n');

        Block {
            kind,
            span: Span::new(start, self.cursor.pos),
        }
    }

    fn parse_paragraph(&mut self) -> Block {
        let start = self.cursor.pos;
        let mut content_end;

        loop {
            self.cursor.consume_while(|c| c != '\n');
            content_end = self.cursor.pos;

            if self.cursor.is_eof() {
                break;
            }
            self.cursor.consume(|c| c == '\n');

            let next = self.cursor.peek();

            match next {
                None | Some('\n') => {
                    break;
                }
                Some(_) if self.is_heading(&self.cursor.src[self.cursor.pos..]) => {
                    break;
                }
                Some(_) => {
                    // continuation line — keep accumulating
                }
            }
        }

        let inline_span = Span::new(start, content_end);
        Block {
            kind: BlockKind::Paragraph {
                children: self.parse_inline(inline_span),
            },
            span: Span::new(start, content_end),
        }
    }

    fn parse_inline(&self, span: Span) -> Vec<Inline> {
        let mut inlines = vec![];

        let mut cur_pos = span.start;

        while cur_pos < span.end {
            let remaining = &self.cursor.src[cur_pos..span.end];

            if let Some(inner) = remaining.strip_prefix("[[") {
                let pipe_pos = inner.find('|');
                let close_pos = inner.find("]]");

                if let Some(close_offset) = close_pos {
                    // check if alias is possible first and make sure pipe comes before close
                    let (target_end, children, total_len) = match pipe_pos {
                        Some(p) if p < close_offset => {
                            // [[target|alias]]
                            let alias_span =
                                Span::new(cur_pos + 2 + p + 1, cur_pos + 2 + close_offset);
                            let kids = self.parse_inline(alias_span);

                            (cur_pos + 2 + p, Some(kids), 2 + close_offset + 2)
                        }
                        _ => {
                            // [[target]]
                            (cur_pos + 2 + close_offset, None, 2 + close_offset + 2)
                        }
                    };

                    let target_span = Span::new(cur_pos + 2, target_end);

                    inlines.push(Inline {
                        kind: InlineKind::Wikilink {
                            target_span,
                            children,
                        },
                        span: Span::new(cur_pos, cur_pos + total_len),
                    });

                    cur_pos += total_len;
                    continue;
                }
            }

            if remaining.starts_with("[")
                && let Some(mid_offset) = remaining.find("](")
            {
                let url_start_offset = mid_offset + 2;
                let remaining_after_mid = &remaining[url_start_offset..];

                if let Some(url_end_offset) = remaining_after_mid.find(")") {
                    let end_offset = url_start_offset + url_end_offset;
                    let total_len = end_offset + 1;

                    let children_span = Span::new(cur_pos + 1, cur_pos + mid_offset);
                    let url_span = Span::new(cur_pos + url_start_offset, cur_pos + end_offset);

                    inlines.push(Inline {
                        kind: InlineKind::Link {
                            children: self.parse_inline(children_span),
                            url_span,
                        },
                        span: Span::new(cur_pos, cur_pos + total_len),
                    });

                    cur_pos += total_len;
                    continue;
                }
            }

            let mut text_len = 0;
            let mut chars = remaining.chars();

            if let Some(first_char) = chars.next() {
                text_len += first_char.len_utf8();
            }

            for c in chars {
                if c == '[' {
                    break;
                }
                text_len += c.len_utf8();
            }

            inlines.push(Inline {
                kind: InlineKind::Text,
                span: Span::new(cur_pos, cur_pos + text_len),
            });

            cur_pos += text_len;
        }

        inlines
    }

    fn is_heading(&self, src: &str) -> bool {
        let trimmed = src.trim_start_matches('#');
        !trimmed.is_empty() && trimmed.starts_with(' ')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_header() {
        let input = "# Header 1";

        let result = Parser::new(input).parse();

        let expected = Document {
            blocks: vec![Block {
                span: Span::new(0, 10),
                kind: BlockKind::Heading {
                    level: 1,
                    children: vec![Inline {
                        kind: InlineKind::Text,
                        span: Span::new(2, 10),
                    }],
                },
            }],
        };

        assert_eq!(result, expected);
    }

    #[test]
    fn parse_paragraph() {
        let input = "hello world";
        let result = Parser::new(input).parse();
        let expected = Document {
            blocks: vec![Block {
                span: Span::new(0, 11),
                kind: BlockKind::Paragraph {
                    children: vec![Inline {
                        kind: InlineKind::Text,
                        span: Span::new(0, 11),
                    }],
                },
            }],
        };
        assert_eq!(result, expected);
    }

    #[test]
    fn parse_multiline_paragraph() {
        let input = "hello world\nhello world again but louder!";
        let result = Parser::new(input).parse();
        let expected = Document {
            blocks: vec![Block {
                span: Span::new(0, 41),
                kind: BlockKind::Paragraph {
                    children: vec![Inline {
                        kind: InlineKind::Text,
                        span: Span::new(0, 41),
                    }],
                },
            }],
        };
        assert_eq!(result, expected);
    }

    #[test]
    fn parse_header_under_paragraph() {
        let input = "hello world\n## Heading 2";
        let result = Parser::new(input).parse();
        let expected = Document {
            blocks: vec![
                Block {
                    span: Span::new(0, 11),
                    kind: BlockKind::Paragraph {
                        children: vec![Inline {
                            kind: InlineKind::Text,
                            span: Span::new(0, 11),
                        }],
                    },
                },
                Block {
                    span: Span::new(12, 24),
                    kind: BlockKind::Heading {
                        level: 2,
                        children: vec![Inline {
                            kind: InlineKind::Text,
                            span: Span::new(15, 24),
                        }],
                    },
                },
            ],
        };
        assert_eq!(result, expected);
    }

    #[test]
    fn parse_multiple_blocks() {
        let input = "# Heading\nhello world";
        let result = Parser::new(input).parse();
        assert_eq!(result.blocks.len(), 2);
        assert!(matches!(
            result.blocks[0].kind,
            BlockKind::Heading { level: 1, .. }
        ));
        assert!(matches!(result.blocks[1].kind, BlockKind::Paragraph { .. }));
    }

    #[test]
    fn parse_multiple_headers() {
        let input = "# Heading\n## Heading 2\n### Heading 3";
        let result = Parser::new(input).parse();
        assert_eq!(result.blocks.len(), 3);
        assert!(matches!(
            result.blocks[0].kind,
            BlockKind::Heading { level: 1, .. }
        ));
        assert!(matches!(
            result.blocks[1].kind,
            BlockKind::Heading { level: 2, .. }
        ));
        assert!(matches!(
            result.blocks[2].kind,
            BlockKind::Heading { level: 3, .. }
        ));
    }

    #[test]
    fn parse_link() {
        let input = "[Google Searh](https://google.com)";
        let result = Parser::new(input).parse();

        let expected = Document {
            blocks: vec![Block {
                span: Span::new(0, 34),
                kind: BlockKind::Paragraph {
                    children: vec![Inline {
                        span: Span::new(0, 34),
                        kind: InlineKind::Link {
                            children: vec![Inline {
                                kind: InlineKind::Text,
                                span: Span::new(1, 13),
                            }],
                            url_span: Span::new(15, 33),
                        },
                    }],
                },
            }],
        };

        assert_eq!(result, expected);
    }

    #[test]
    fn parse_wikilink() {
        let input = "[[../other_file.rs]]";
        let result = Parser::new(input).parse();

        let expected = Document {
            blocks: vec![Block {
                span: Span::new(0, 20),
                kind: BlockKind::Paragraph {
                    children: vec![Inline {
                        span: Span::new(0, 20),
                        kind: InlineKind::Wikilink {
                            target_span: Span::new(2, 18),
                            children: None,
                        },
                    }],
                },
            }],
        };

        assert_eq!(result, expected);
    }

    #[test]
    fn parse_wikilink_with_children() {
        let input = "[[../other_file.rs|Other File]]";
        let result = Parser::new(input).parse();

        if let BlockKind::Paragraph { children } = &result.blocks[0].kind {
            if let InlineKind::Wikilink {
                children: wikilink_children,
                ..
            } = &children[0].kind
            {
                assert!(wikilink_children.is_some());
                if let Some(child_inlines) = wikilink_children {
                    assert!(!child_inlines.is_empty());
                }
            } else {
                panic!("Expected Wikilink");
            }
        } else {
            panic!("Expected Paragraph");
        }
    }
}
