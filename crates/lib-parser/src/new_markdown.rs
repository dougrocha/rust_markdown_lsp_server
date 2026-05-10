#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        assert!(start <= end, "Span::new: start ({start}) > end ({end})");
        Self { start, end }
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn as_str<'a>(&self, src: &'a str) -> &'a str {
        src.get(self.start..self.end).unwrap_or_default()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ListType {
    Unordered,
    Ordered,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockKind {
    Heading {
        level: u8,
        children: Vec<Inline>,
    },
    Paragraph {
        children: Vec<Inline>,
    },
    List {
        kind: ListType,
        children: Vec<Block>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub kind: BlockKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InlineKind {
    Text,
    Bold {
        children: Vec<Inline>,
    },
    Italic {
        children: Vec<Inline>,
    },
    BoldItalic {
        children: Vec<Inline>,
    },
    Strikethrough {
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
    Footnote {
        identifier: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Inline {
    pub kind: InlineKind,
    pub span: Span,
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
    #[inline]
    fn abs_pos(&self) -> usize {
        self.pos + self.offset
    }

    fn next(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn consume_escaped(&mut self) -> Option<Span> {
        if !self.src[self.pos..].starts_with('\\') {
            return None;
        }

        let next = self.src[self.pos + 1..].chars().next()?;
        if !next.is_ascii_punctuation() {
            return None;
        }
        let start = self.abs_pos();

        self.next(); // consume '\'
        self.next(); // consume the escaped char

        Some(Span::new(start, self.abs_pos()))
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
    fn find_abs(&self, c: char) -> Option<usize> {
        self.src[self.pos..].find(c).map(|i| self.abs_pos() + i)
    }

    fn is_heading(&self) -> bool {
        let trimmed = self.src[self.pos..].trim_start_matches('#');
        !trimmed.is_empty() && trimmed.starts_with(' ')
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

    pub fn parse(mut self) -> Vec<Block> {
        let mut blocks = Vec::new();

        while !self.cursor.is_eof() {
            if self.cursor.consume_if(|c| c == '\n') {
                continue;
            }

            blocks.push(self.parse_block());
        }

        blocks
    }

    fn parse_block(&mut self) -> Block {
        match self.cursor.peek() {
            Some('#') => self.parse_heading(),
            Some(_) => self.parse_paragraph(),
            None => unreachable!("parse_block called at EOF; caller must check is_eof() first"),
        }
    }

    fn parse_heading(&mut self) -> Block {
        let start = self.cursor.pos;

        let level_span = self.cursor.consume_while(|c| c == '#');
        let spaces = self.cursor.consume_while(|c| c == ' ');

        if spaces.is_empty() {
            self.cursor.pos = start;
            return self.parse_paragraph();
        }

        let inline_span = Span::new(
            self.cursor.abs_pos(),
            self.cursor.consume_while(|c| c != '\n').end,
        );

        let kind = BlockKind::Heading {
            level: (level_span.len() as u8).min(6),
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
                None | Some('\n') => break,
                Some(_) if self.cursor.is_heading() => break,

                // continuation line — keep accumulating
                Some(_) => continue,
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

        let mut inlines = Vec::new();

        // Start of the current loop of plain text
        // If None = no text yet
        let mut text_start: Option<usize> = None;

        while !local_cursor.is_eof() {
            let abs = local_cursor.abs_pos();

            // consume escaped characters as normal text
            if local_cursor.consume_escaped().is_some() {
                text_start.get_or_insert(abs);
                continue;
            }

            let parsed_inline = self.try_inline_at(&mut local_cursor, span, abs);

            if let Some((kind, total_span)) = parsed_inline {
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

    fn try_inline_at(
        &self,
        cursor: &mut Cursor<'_>,
        span: Span,
        abs: usize,
    ) -> Option<(InlineKind, Span)> {
        let src = self.cursor.src;

        if cursor.starts_with("[[") {
            let line_end = cursor.find_abs('\n').unwrap_or(span.end);
            let (target_span, alias_span, total_span) =
                Self::try_parse_wikilink(&src[..line_end], abs)?;
            let children = alias_span.map(|s| self.parse_inline(s));

            return Some((
                InlineKind::Wikilink {
                    target_span,
                    children,
                },
                total_span,
            ));
        }

        if cursor.starts_with("[^") {
            let local_line_end = cursor.find_abs('\n').unwrap_or(span.end);

            let (identifier_span, total_span) =
                Self::try_parse_footnote(&src[..local_line_end], abs)?;

            return Some((
                InlineKind::Footnote {
                    identifier: identifier_span,
                },
                total_span,
            ));
        }

        if cursor.starts_with("[") {
            let (children_span, url_span, total_span) =
                Self::try_parse_link(&src[..span.end], abs)?;

            return Some((
                InlineKind::Link {
                    children: self.parse_inline(children_span),
                    url_span,
                },
                total_span,
            ));
        }

        if cursor.starts_with("[") {
            let (children_span, url_span, total_span) =
                Self::try_parse_link(&src[..span.end], abs)?;
            let children = self.parse_inline(children_span);

            return Some((InlineKind::Link { children, url_span }, total_span));
        }

        // Bold-italic must be checked before bold (longer prefix wins).
        if cursor.starts_with("***") || cursor.starts_with("___") {
            let (children_span, total_span) =
                Self::try_parse_bold_italic_text(&src[..span.end], abs)?;
            let children = self.parse_inline(children_span);

            return Some((InlineKind::BoldItalic { children }, total_span));
        }

        if cursor.starts_with("**") || cursor.starts_with("__") {
            let (children_span, total_span) = Self::try_parse_bold_text(&src[..span.end], abs)?;
            let children = self.parse_inline(children_span);

            return Some((InlineKind::Bold { children }, total_span));
        }

        if cursor.starts_with("~~") {
            let (children_span, total_span) = Self::try_parse_strikethrough(&src[..span.end], abs)?;
            let children = self.parse_inline(children_span);

            return Some((InlineKind::Strikethrough { children }, total_span));
        }

        if cursor.starts_with("*") || cursor.starts_with("_") {
            let (children_span, total_span) = Self::try_parse_italic_text(&src[..span.end], abs)?;
            let children = self.parse_inline(children_span);

            return Some((InlineKind::Italic { children }, total_span));
        }
        None
    }

    /// Tries to get a Spans for a wikilink.
    ///
    /// # Returns
    ///
    /// `Some((target_span, alias_span, total_span))` where:
    ///
    /// * `target_span` covers the link target (between `[[` and `|` or `]]`).
    /// * `alias_span` covers the display alias when a `|` separator is present.
    /// * `total_span` covers the full `[[…]]` construct.
    ///
    /// Returns `None` when no valid wikilink is found.
    fn try_parse_wikilink(src: &str, pos: usize) -> Option<(Span, Option<Span>, Span)> {
        let content_start = pos + 2; // "[["
        let rest = &src[content_start..];

        let close_offset = rest.find("]]")?;
        let content = &rest[..close_offset];

        // reject nested link: "[[" means we start over
        if content.contains("[[") {
            return None;
        }

        let content_end = close_offset + content_start;

        let (target_end, alias_span) = match content.find('|') {
            Some(pipe_offset) => {
                let alias_start = content_start + pipe_offset + 1;
                (
                    content_start + pipe_offset,
                    Some(Span::new(alias_start, content_end)),
                )
            }
            None => (content_end, None),
        };

        let target_span = Span::new(content_start, target_end);
        let total_span = Span::new(pos, pos + content_end + 2 - pos);

        Some((target_span, alias_span, total_span))
    }

    /// Try to extract a link.
    ///
    /// # Returns
    ///
    /// `Some((children_span, url_span, total_span))` on success, `None`
    /// otherwise.
    fn try_parse_link(src: &str, pos: usize) -> Option<(Span, Span, Span)> {
        let content_start = pos + 1; // skips "["
        let remaining = &src[content_start..];

        let mid_offset = remaining.find("](")?;
        let children_text = &src[content_start..content_start + mid_offset];

        // reject nested links
        if children_text.contains('[') {
            return None;
        }

        let url_start = content_start + mid_offset + 2; // +2 skips "]("

        let url_remaining = &src[url_start..];
        let url_end_offset = url_remaining.find(')')?;
        let url_end_abs = url_start + url_end_offset;

        let url_text = &src[url_start..url_end_abs];
        if url_text.contains('[') {
            return None;
        }

        let children_span = Span::new(content_start, content_start + mid_offset);
        let url_span = Span::new(url_start, url_end_abs);
        let total_span = Span::new(pos, url_end_abs + 1);

        Some((children_span, url_span, total_span))
    }

    fn try_parse_delimited(src: &str, pos: usize, delim: &str) -> Option<(Span, Span)> {
        if !src[pos..].starts_with(delim) {
            return None;
        }
        let len = delim.len();
        let start_pos = pos + len;
        let end = src[start_pos..].find(delim).map(|i| start_pos + i)?;
        if start_pos == end {
            return None;
        }
        Some((Span::new(start_pos, end), Span::new(pos, end + len)))
    }

    fn try_parse_bold_text(src: &str, pos: usize) -> Option<(Span, Span)> {
        let delim = if src[pos..].starts_with("**") {
            "**"
        } else if src[pos..].starts_with("__") {
            "__"
        } else {
            return None;
        };
        Self::try_parse_delimited(src, pos, delim)
    }

    fn try_parse_italic_text(src: &str, pos: usize) -> Option<(Span, Span)> {
        let delim = if src[pos..].starts_with('*') {
            "*"
        } else if src[pos..].starts_with('_') {
            "_"
        } else {
            return None;
        };
        Self::try_parse_delimited(src, pos, delim)
    }

    fn try_parse_bold_italic_text(src: &str, pos: usize) -> Option<(Span, Span)> {
        let delim = if src[pos..].starts_with("***") {
            "***"
        } else if src[pos..].starts_with("___") {
            "___"
        } else {
            return None;
        };
        Self::try_parse_delimited(src, pos, delim)
    }

    fn try_parse_strikethrough(src: &str, pos: usize) -> Option<(Span, Span)> {
        Self::try_parse_delimited(src, pos, "~~")
    }

    fn try_parse_footnote(src: &str, pos: usize) -> Option<(Span, Span)> {
        let ident_start = pos + 2;
        let ident_end = src[2..].find(']').map(|i| ident_start + i)?;

        if ident_start == ident_end {
            return None;
        }

        if src
            .get(2..ident_end - pos)?
            .chars()
            .any(|c| c.is_ascii_whitespace())
        {
            return None;
        }

        Some((
            Span::new(ident_start, ident_end),
            Span::new(pos, ident_end + 1),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_header() {
        let input = "# Header 1";

        let result = Parser::new(input).parse();

        let expected = vec![Block {
            span: Span::new(0, 10),
            kind: BlockKind::Heading {
                level: 1,
                children: vec![Inline {
                    kind: InlineKind::Text,
                    span: Span::new(2, 10),
                }],
            },
        }];

        assert_eq!(result, expected);
    }

    #[test]
    fn parse_header_with_link() {
        let input = "# Go here [Google](google.com)";

        let result = Parser::new(input).parse();

        let expected = vec![Block {
            span: Span::new(0, 30),
            kind: BlockKind::Heading {
                level: 1,
                children: vec![
                    Inline {
                        kind: InlineKind::Text,
                        span: Span::new(2, 10),
                    },
                    Inline {
                        kind: InlineKind::Link {
                            children: vec![Inline {
                                kind: InlineKind::Text,
                                span: Span::new(11, 17),
                            }],
                            url_span: Span::new(19, 29),
                        },
                        span: Span::new(10, 30),
                    },
                ],
            },
        }];

        assert_eq!(result, expected);
    }

    #[test]
    fn parse_paragraph() {
        let input = "hello world";
        let result = Parser::new(input).parse();
        let expected = vec![Block {
            span: Span::new(0, 11),
            kind: BlockKind::Paragraph {
                children: vec![Inline {
                    kind: InlineKind::Text,
                    span: Span::new(0, 11),
                }],
            },
        }];
        assert_eq!(result, expected);
    }

    #[test]
    fn parse_multiline_paragraph() {
        let input = "hello world\nhello world again but louder!";
        let result = Parser::new(input).parse();
        let expected = vec![Block {
            span: Span::new(0, 41),
            kind: BlockKind::Paragraph {
                children: vec![Inline {
                    kind: InlineKind::Text,
                    span: Span::new(0, 41),
                }],
            },
        }];
        assert_eq!(result, expected);
    }

    #[test]
    fn parse_header_under_paragraph() {
        let input = "hello world\n## Heading 2";
        let result = Parser::new(input).parse();
        let expected = vec![
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
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn parse_multiple_blocks() {
        let input = "# Heading\nhello world";
        let result = Parser::new(input).parse();
        assert_eq!(result.len(), 2);
        assert!(matches!(
            result[0].kind,
            BlockKind::Heading { level: 1, .. }
        ));
        assert!(matches!(result[1].kind, BlockKind::Paragraph { .. }));
    }

    #[test]
    fn parse_multiple_headers() {
        let input = "# Heading\n## Heading 2\n### Heading 3";
        let result = Parser::new(input).parse();
        assert_eq!(result.len(), 3);
        assert!(matches!(
            result[0].kind,
            BlockKind::Heading { level: 1, .. }
        ));
        assert!(matches!(
            result[1].kind,
            BlockKind::Heading { level: 2, .. }
        ));
        assert!(matches!(
            result[2].kind,
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

        let expected = vec![Block {
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
        }];

        assert_eq!(result, expected);
    }

    #[test]
    fn parse_wikilink() {
        let input = "[[../other_file.rs]]";
        let result = Parser::new(input).parse();

        let expected = vec![Block {
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
        }];

        assert_eq!(result, expected);
    }

    #[test]
    fn parse_broken_wikilink() {
        let input = "[[../other_file";
        let result = Parser::new(input).parse();

        let expected = vec![Block {
            span: Span::new(0, 15),
            kind: BlockKind::Paragraph {
                children: vec![Inline {
                    span: Span::new(0, 15),
                    kind: InlineKind::Text,
                }],
            },
        }];

        assert_eq!(result, expected);
    }

    #[test]
    fn parse_nested_broken_wikilink() {
        let input = "[[broken link [[fixed link]]";
        let result = Parser::new(input).parse();

        let expected = vec![Block {
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
        }];

        assert_eq!(result, expected);
    }

    #[test]
    fn parse_broken_wikilink_with_link_syntax() {
        let input = "[[]()";
        let result = Parser::new(input).parse();

        let expected = vec![Block {
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
        }];

        assert_eq!(result, expected);
    }

    #[test]
    fn parse_broken_link_then_valid_link() {
        let input = "[x [a](b)";
        let result = Parser::new(input).parse();

        let expected = vec![Block {
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
        }];

        assert_eq!(result, expected);
    }

    #[test]
    fn parse_broken_link_with_url_bracket() {
        let input = "[x](fake [a](b)";
        let result = Parser::new(input).parse();

        let expected = vec![Block {
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
        }];

        assert_eq!(result, expected);
    }

    #[test]
    fn parse_wikilink_with_children() {
        let input = "[[../other_file.rs|Other File]]";
        let result = Parser::new(input).parse();

        if let BlockKind::Paragraph { children } = &result[0].kind {
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

        let expected = vec![Block {
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
        }];

        assert_eq!(result, expected);
    }

    #[test]
    fn parse_empty_bold() {
        let input = "****";
        let result = Parser::new(input).parse();

        let expected = vec![Block {
            span: Span::new(0, 4),
            kind: BlockKind::Paragraph {
                children: vec![Inline {
                    span: Span::new(0, 4),
                    kind: InlineKind::Text,
                }],
            },
        }];

        assert_eq!(result, expected);
    }

    #[test]
    fn parse_bold_with_children() {
        let input = "**bold [link](http://example.com) text**";
        let result = Parser::new(input).parse();

        let expected = vec![Block {
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
        }];

        assert_eq!(result, expected);
    }

    #[test]
    fn parse_bold_text_mid_string() {
        let res = Parser::try_parse_bold_text("hi **x** y", 3);
        assert_eq!(res, Some((Span::new(5, 6), Span::new(3, 8))));
    }

    #[test]
    fn parse_footnote_mid_string() {
        let input = "see [^abc] more";
        let result = Parser::new(input).parse();

        let expected = vec![Block {
            kind: BlockKind::Paragraph {
                children: vec![Inline {
                    kind: InlineKind::Text,
                    span: Span::new(0, 15),
                }],
            },
            span: Span::new(0, 15),
        }];

        assert_eq!(result, expected);
    }

    #[test]
    fn parse_footnote_rejects_whitespace_mid_string() {
        let input = "see [^a b] more";
        let result = Parser::new(input).parse();

        let expected = vec![Block {
            kind: BlockKind::Paragraph {
                children: vec![Inline {
                    kind: InlineKind::Text,
                    span: Span::new(0, 15),
                }],
            },
            span: Span::new(0, 15),
        }];

        assert_eq!(result, expected);
    }

    #[test]
    fn parse_escape_bold_and_heading() {
        let input = r"\*\*not bold\*\*   \# not a heading";
        let result = Parser::new(input).parse();

        let expected = vec![Block {
            kind: BlockKind::Paragraph {
                children: vec![Inline {
                    kind: InlineKind::Text,
                    span: Span::new(0, 35),
                }],
            },
            span: Span::new(0, 35),
        }];

        assert_eq!(result, expected);
    }

    #[test]
    fn parse_italic_star() {
        let input = "*italic*";
        let result = Parser::new(input).parse();

        let expected = vec![Block {
            kind: BlockKind::Paragraph {
                children: vec![Inline {
                    kind: InlineKind::Italic {
                        children: vec![Inline {
                            kind: InlineKind::Text,
                            span: Span::new(1, 7),
                        }],
                    },
                    span: Span::new(0, 8),
                }],
            },
            span: Span::new(0, 8),
        }];

        assert_eq!(result, expected);
    }

    #[test]
    fn parse_italic_underscore() {
        let input = "_italic_";
        let result = Parser::new(input).parse();

        let expected = vec![Block {
            kind: BlockKind::Paragraph {
                children: vec![Inline {
                    kind: InlineKind::Italic {
                        children: vec![Inline {
                            kind: InlineKind::Text,
                            span: Span::new(1, 7),
                        }],
                    },
                    span: Span::new(0, 8),
                }],
            },
            span: Span::new(0, 8),
        }];

        assert_eq!(result, expected);
    }

    #[test]
    fn parse_bold_underscore() {
        let input = "__bold__";
        let result = Parser::new(input).parse();

        let expected = vec![Block {
            kind: BlockKind::Paragraph {
                children: vec![Inline {
                    kind: InlineKind::Bold {
                        children: vec![Inline {
                            kind: InlineKind::Text,
                            span: Span::new(2, 6),
                        }],
                    },
                    span: Span::new(0, 8),
                }],
            },
            span: Span::new(0, 8),
        }];

        assert_eq!(result, expected);
    }

    #[test]
    fn parse_bold_italic_triple_star() {
        let input = "***bold italic***";
        let result = Parser::new(input).parse();

        let expected = vec![Block {
            kind: BlockKind::Paragraph {
                children: vec![Inline {
                    kind: InlineKind::BoldItalic {
                        children: vec![Inline {
                            kind: InlineKind::Text,
                            span: Span::new(3, 14),
                        }],
                    },
                    span: Span::new(0, 17),
                }],
            },
            span: Span::new(0, 17),
        }];

        assert_eq!(result, expected);
    }

    #[test]
    fn parse_bold_italic_triple_underscore() {
        let input = "___bold italic___";
        let result = Parser::new(input).parse();

        let expected = vec![Block {
            kind: BlockKind::Paragraph {
                children: vec![Inline {
                    kind: InlineKind::BoldItalic {
                        children: vec![Inline {
                            kind: InlineKind::Text,
                            span: Span::new(3, 14),
                        }],
                    },
                    span: Span::new(0, 17),
                }],
            },
            span: Span::new(0, 17),
        }];

        assert_eq!(result, expected);
    }

    #[test]
    fn parse_strikethrough() {
        let input = "~~strikethrough~~";
        let result = Parser::new(input).parse();

        let expected = vec![Block {
            kind: BlockKind::Paragraph {
                children: vec![Inline {
                    kind: InlineKind::Strikethrough {
                        children: vec![Inline {
                            kind: InlineKind::Text,
                            span: Span::new(2, 15),
                        }],
                    },
                    span: Span::new(0, 17),
                }],
            },
            span: Span::new(0, 17),
        }];

        assert_eq!(result, expected);
    }

    #[test]
    fn parse_empty_italic() {
        let input = "**";
        let result = Parser::new(input).parse();

        let expected = vec![Block {
            kind: BlockKind::Paragraph {
                children: vec![Inline {
                    kind: InlineKind::Text,
                    span: Span::new(0, 2),
                }],
            },
            span: Span::new(0, 2),
        }];

        assert_eq!(result, expected);
    }

    #[test]
    fn parse_empty_bold_italic() {
        let input = "******";
        let result = Parser::new(input).parse();

        let expected = vec![Block {
            kind: BlockKind::Paragraph {
                children: vec![Inline {
                    kind: InlineKind::Text,
                    span: Span::new(0, 6),
                }],
            },
            span: Span::new(0, 6),
        }];

        assert_eq!(result, expected);
    }

    #[test]
    fn parse_empty_strikethrough() {
        let input = "~~~~";
        let result = Parser::new(input).parse();

        let expected = vec![Block {
            kind: BlockKind::Paragraph {
                children: vec![Inline {
                    kind: InlineKind::Text,
                    span: Span::new(0, 4),
                }],
            },
            span: Span::new(0, 4),
        }];

        assert_eq!(result, expected);
    }
}
