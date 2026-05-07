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

    pub fn as_str<'a>(&self, src: &'a str) -> &'a str {
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

    /// Offset if using recursive cursor, this offset is the byte offset from the original source.
    offset: usize,
}

impl<'src> Cursor<'src> {
    fn new(source: &'src str) -> Self {
        Self {
            src: source,
            pos: 0,
            offset: 0,
        }
    }

    fn with_offset(source: &'src str, offset: usize) -> Self {
        Self {
            src: source,
            pos: 0,
            offset,
        }
    }

    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.src.len()
    }

    /// Gets the current byte position in original source
    fn abs_pos(&self) -> usize {
        self.pos + self.offset
    }

    fn next(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn consume_span(&mut self, span: Span) {
        self.pos += span.len();
    }

    fn consume_if<F>(&mut self, f: F) -> bool
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
        let start = self.abs_pos();
        while self.peek().is_some_and(&f) {
            self.next();
        }
        Span::new(start, self.abs_pos())
    }

    fn starts_with(&self, pat: &str) -> bool {
        self.src[self.pos..].starts_with(pat)
    }

    /// Returns the absolute position of the first occurrence of `c`
    /// at or after the current position, or `None` if not found.
    fn find(&self, c: char) -> Option<usize> {
        self.src[self.pos..].find(c).map(|i| self.abs_pos() + i)
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
            if self.cursor.consume_if(|c| c == '\n') {
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
            None => panic!("We should never panic here because we handle eof elsewhere"),
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
            level: level.min(6) as u8,
            children: self.parse_inline(inline_span),
        };

        self.cursor.consume_if(|c| c == '\n');

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
            self.cursor.consume_if(|c| c == '\n');

            let next = self.cursor.peek();

            match next {
                None | Some('\n') => {
                    break;
                }
                Some(_) if Self::is_heading(&self.cursor.src[self.cursor.pos..]) => {
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
        let src_slice = span.as_str(self.cursor.src);
        let mut local_cursor = Cursor::with_offset(src_slice, span.start);

        let mut inlines = vec![];

        let mut text_start = None;

        while !local_cursor.is_eof() {
            let abs = local_cursor.abs_pos();

            let inline_element = if local_cursor.starts_with("[[") {
                let local_line_end = local_cursor.find('\n').unwrap_or(span.end);

                Self::try_parse_wikilink(&self.cursor.src[..local_line_end], abs).map(
                    |(target_span, alias_span, total_span)| {
                        let children = alias_span.map(|s| self.parse_inline(s));
                        (
                            InlineKind::Wikilink {
                                target_span,
                                children,
                            },
                            total_span,
                        )
                    },
                )
            } else if local_cursor.starts_with("[") {
                Self::try_parse_link(&self.cursor.src[..span.end], abs).map(
                    |(children_span, url_span, total_span)| {
                        (
                            InlineKind::Link {
                                children: self.parse_inline(children_span),
                                url_span,
                            },
                            total_span,
                        )
                    },
                )
            } else if local_cursor.starts_with("**") {
                Self::try_parse_bold_text(&self.cursor.src[..span.end], abs).map(
                    |(children_span, total_span)| {
                        (
                            InlineKind::Bold {
                                children: self.parse_inline(children_span),
                            },
                            total_span,
                        )
                    },
                )
            } else {
                None
            };

            if let Some((kind, total_span)) = inline_element {
                if let Some(start) = text_start.take() {
                    inlines.push(Inline {
                        kind: InlineKind::Text,
                        span: Span::new(start, abs),
                    });
                }
                inlines.push(Inline {
                    kind,
                    span: total_span,
                });
                local_cursor.consume_span(total_span);
            } else {
                if text_start.is_none() {
                    text_start = Some(abs);
                }
                local_cursor.next();
            }
        }

        if let Some(start) = text_start.take() {
            inlines.push(Inline {
                kind: InlineKind::Text,
                span: Span::new(start, local_cursor.abs_pos()),
            });
        }

        inlines
    }

    /// Tries to get a Spans for a wikilink.
    ///
    /// # Arguments
    ///
    /// * `src` - A string slice for where to search.
    /// * `pos` - Starting position to parse from.
    ///
    /// # Returns
    ///
    /// Return an [`Option`] containing these items,
    /// or [`None`] if wikilink is not found
    /// * `target_span`
    /// * `alias_span`
    /// * `total_span`
    /// * `new_pos`
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let input = "[[Page Title|Display Text]]";
    /// let (target, alias, total, new_pos) = Parser::try_parse_wikilink(input, 0).unwrap();
    /// assert_eq!(target.as_str(input), "Page Title");
    /// assert_eq!(alias.map(|s| s.as_str(input)), Some("Display Text"));
    /// assert_eq!(new_pos, 27);
    /// ```
    fn try_parse_wikilink(src: &str, pos: usize) -> Option<(Span, Option<Span>, Span)> {
        let search_start = pos + 2;
        let rest = &src[search_start..];

        let close_offset = rest.find("]]")?;
        let close_abs = search_start + close_offset;

        let content = &src[search_start..close_abs];

        if content.contains("[[") {
            return None;
        }

        let pipe_offset = content.find('|');

        let (target_end, alias_span, total_len) = match pipe_offset {
            Some(p) => {
                let alias_start = search_start + p + 1;
                let alias_span = Span::new(alias_start, close_abs);
                (search_start + p, Some(alias_span), close_abs + 2 - pos)
            }
            None => (close_abs, None, close_abs + 2 - pos),
        };

        let target_span = Span::new(search_start, target_end);
        let total_span = Span::new(pos, pos + total_len);

        Some((target_span, alias_span, total_span))
    }

    /// Try to extract a link.
    ///
    /// # Arguments
    ///
    /// * `src` - A string slice for where to search.
    /// * `pos` - Starting position to parse from.
    ///
    /// # Returns
    ///
    /// `Some((children_span, url_span, total_span, new_pos))` when a link is
    /// found, where:
    ///
    /// * `children_span` – Span covering the link text between `[` and `]`.
    /// * `url_span` – Span covering the URL between `(` and `)`.
    /// * `total_span` – Span covering the entire `[text](url)` construct.
    /// * `new_pos` – Byte offset immediately after the closing `)`.
    ///
    /// `None` if a link is not possible.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let input = "[Google](https://google.com)";
    /// let (children, url, total, new_pos) = Parser::try_parse_link(input, 0).unwrap();
    /// assert_eq!(children.as_str(input), "Google");
    /// assert_eq!(url.as_str(input), "https://google.com");
    /// assert_eq!(new_pos, 28);
    /// ```
    fn try_parse_link(src: &str, pos: usize) -> Option<(Span, Span, Span)> {
        let search_start = pos + 1;
        let remaining = &src[search_start..];

        let mid_offset = remaining.find("](")?;

        let children_text = &src[search_start..search_start + mid_offset];
        if children_text.contains('[') {
            return None;
        }

        let url_start_abs = search_start + mid_offset + 2;

        let url_remaining = &src[url_start_abs..];
        let url_end_offset = url_remaining.find(')')?;
        let url_end_abs = url_start_abs + url_end_offset;
        let total_end = url_end_abs + 1;

        let url_text = &src[url_start_abs..url_end_abs];
        if url_text.contains('[') {
            return None;
        }

        let children_span = Span::new(search_start, search_start + mid_offset);
        let url_span = Span::new(url_start_abs, url_end_abs);
        let total_span = Span::new(pos, total_end);

        Some((children_span, url_span, total_span))
    }

    fn try_parse_bold_text(src: &str, pos: usize) -> Option<(Span, Span)> {
        if !src[pos..].starts_with("**") {
            return None;
        }

        let start_pos = pos + 2;
        let remaining = &src[start_pos..];

        let bold_end_offset = remaining.find("**")?;
        let end = start_pos + bold_end_offset + 2;

        Some((
            Span::new(start_pos, start_pos + bold_end_offset),
            Span::new(pos, end),
        ))
    }

    fn is_heading(src: &str) -> bool {
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
    fn try_parse_link_basic() {
        let input = "[Google Search](https://google.com)";
        let result = Parser::try_parse_link(input, 0);

        assert_eq!(
            result,
            Some((Span::new(1, 14), Span::new(16, 34), Span::new(0, 35)))
        );
    }

    #[test]
    fn try_parse_link_mid_string() {
        let input = "text [link](url) more";
        let result = Parser::try_parse_link(input, 5);

        assert_eq!(
            result,
            Some((Span::new(6, 10), Span::new(12, 15), Span::new(5, 16)))
        );
    }

    #[test]
    fn try_parse_wikilink_basic() {
        let input = "[[../other_file.rs]]";
        let result = Parser::try_parse_wikilink(input, 0);

        assert_eq!(result, Some((Span::new(2, 18), None, Span::new(0, 20))));
    }

    #[test]
    fn try_parse_wikilink_with_alias() {
        let input = "[[../other_file.rs|Other File]]";
        let result = Parser::try_parse_wikilink(input, 0);

        assert_eq!(
            result,
            Some((Span::new(2, 18), Some(Span::new(19, 29)), Span::new(0, 31)))
        );
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
    fn parse_broken_wikilink() {
        let input = "[[../other_file";
        let result = Parser::new(input).parse();

        let expected = Document {
            blocks: vec![Block {
                span: Span::new(0, 15),
                kind: BlockKind::Paragraph {
                    children: vec![Inline {
                        span: Span::new(0, 15),
                        kind: InlineKind::Text,
                    }],
                },
            }],
        };

        assert_eq!(result, expected);
    }

    #[test]
    fn parse_nested_broken_wikilink() {
        let input = "[[broken link [[fixed link]]";
        let result = Parser::new(input).parse();

        let expected = Document {
            blocks: vec![Block {
                span: Span::new(0, 28),
                kind: BlockKind::Paragraph {
                    children: vec![
                        Inline {
                            span: Span::new(0, 14),
                            kind: InlineKind::Text,
                        },
                        Inline {
                            span: Span::new(14, 28),
                            kind: InlineKind::Wikilink {
                                target_span: Span::new(16, 26),
                                children: None,
                            },
                        },
                    ],
                },
            }],
        };

        assert_eq!(result, expected);
    }

    #[test]
    fn parse_broken_wikilink_with_link_syntax() {
        let input = "[[]()";
        let result = Parser::new(input).parse();

        let expected = Document {
            blocks: vec![Block {
                span: Span::new(0, 5),
                kind: BlockKind::Paragraph {
                    children: vec![
                        Inline {
                            span: Span::new(0, 1),
                            kind: InlineKind::Text,
                        },
                        Inline {
                            span: Span::new(1, 5),
                            kind: InlineKind::Link {
                                children: vec![],
                                url_span: Span::new(4, 4),
                            },
                        },
                    ],
                },
            }],
        };

        assert_eq!(result, expected);
    }

    #[test]
    fn parse_broken_link_then_valid_link() {
        let input = "[x [a](b)";
        let result = Parser::new(input).parse();

        let expected = Document {
            blocks: vec![Block {
                span: Span::new(0, 9),
                kind: BlockKind::Paragraph {
                    children: vec![
                        Inline {
                            span: Span::new(0, 3),
                            kind: InlineKind::Text,
                        },
                        Inline {
                            span: Span::new(3, 9),
                            kind: InlineKind::Link {
                                children: vec![Inline {
                                    kind: InlineKind::Text,
                                    span: Span::new(4, 5),
                                }],
                                url_span: Span::new(7, 8),
                            },
                        },
                    ],
                },
            }],
        };

        assert_eq!(result, expected);
    }

    #[test]
    fn parse_broken_link_with_url_bracket() {
        let input = "[x](fake [a](b)";
        let result = Parser::new(input).parse();

        let expected = Document {
            blocks: vec![Block {
                span: Span::new(0, 15),
                kind: BlockKind::Paragraph {
                    children: vec![
                        Inline {
                            span: Span::new(0, 9),
                            kind: InlineKind::Text,
                        },
                        Inline {
                            span: Span::new(9, 15),
                            kind: InlineKind::Link {
                                children: vec![Inline {
                                    kind: InlineKind::Text,
                                    span: Span::new(10, 11),
                                }],
                                url_span: Span::new(13, 14),
                            },
                        },
                    ],
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

    #[test]
    fn parse_bold_text() {
        let input = "**Bold Text**";
        let result = Parser::new(input).parse();

        let expected = Document {
            blocks: vec![Block {
                span: Span::new(0, 13),
                kind: BlockKind::Paragraph {
                    children: vec![Inline {
                        span: Span::new(0, 13),
                        kind: InlineKind::Bold {
                            children: vec![Inline {
                                kind: InlineKind::Text,
                                span: Span::new(2, 11),
                            }],
                        },
                    }],
                },
            }],
        };

        assert_eq!(result, expected);
    }

    #[test]
    fn parse_bold_with_children() {
        let input = "**bold [link](http://example.com) text**";
        let result = Parser::new(input).parse();

        let expected = Document {
            blocks: vec![Block {
                span: Span::new(0, 40),
                kind: BlockKind::Paragraph {
                    children: vec![Inline {
                        span: Span::new(0, 40),
                        kind: InlineKind::Bold {
                            children: vec![
                                Inline {
                                    kind: InlineKind::Text,
                                    span: Span::new(2, 7),
                                },
                                Inline {
                                    kind: InlineKind::Link {
                                        children: vec![Inline {
                                            kind: InlineKind::Text,
                                            span: Span::new(8, 12),
                                        }],
                                        url_span: Span::new(14, 32),
                                    },
                                    span: Span::new(7, 33),
                                },
                                Inline {
                                    kind: InlineKind::Text,
                                    span: Span::new(33, 38),
                                },
                            ],
                        },
                    }],
                },
            }],
        };

        assert_eq!(result, expected);
    }
}
