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

    /// Returns true if this span fully contains `other`.
    pub fn contains(&self, other: Self) -> bool {
        self.start <= other.start && other.end <= self.end
    }

    /// Returns true if this span contains the given byte offset.
    pub fn contains_offset(&self, offset: usize) -> bool {
        self.start <= offset && offset < self.end
    }
}

impl From<Span> for std::ops::Range<usize> {
    fn from(span: Span) -> Self {
        span.start..span.end
    }
}

impl From<std::ops::Range<usize>> for Span {
    fn from(range: std::ops::Range<usize>) -> Self {
        Span::new(range.start, range.end)
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
    ListItem {
        checked: Option<bool>,
        inline: Vec<Inline>,
        children: Vec<Block>,
    },
    Frontmatter,
    FootnoteDefinition {
        identifier: Span,
        children: Vec<Inline>,
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
    Image {
        alt_span: Span,
        url_span: Span,
    },
    Wikilink {
        target_span: Span,
        children: Option<Vec<Inline>>,
    },
    Footnote {
        identifier: Span,
    },
    Tag {
        name: Span,
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

    /// Number of leading ASCII spaces from the current position.
    fn leading_spaces(&self) -> usize {
        self.src[self.pos..]
            .chars()
            .take_while(|&c| c == ' ')
            .count()
    }

    /// The rest of the current line (not including the newline).
    fn rest_of_line(&self) -> &'src str {
        let rest = &self.src[self.pos..];
        let end = rest.find('\n').unwrap_or(rest.len());
        &rest[..end]
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

        if let Some(frontmatter) = self.try_parse_frontmatter() {
            blocks.push(frontmatter);
        }

        while !self.cursor.is_eof() {
            if self.cursor.consume_if(|c| c == '\n') {
                continue;
            }

            blocks.push(self.parse_block());
        }

        blocks
    }

    /// Recognizes a leading YAML frontmatter block: `---`, a body, then a
    /// closing `---` line. Only valid at the very start of the document.
    fn try_parse_frontmatter(&mut self) -> Option<Block> {
        let src = self.cursor.src;
        if self.cursor.pos != 0 || !src.starts_with("---\n") {
            return None;
        }

        let mut search_pos = 4; // past "---\n"
        loop {
            let rest = &src[search_pos..];
            let line_end = rest.find('\n').map_or(src.len(), |i| search_pos + i);
            let line = &src[search_pos..line_end];

            if line == "---" {
                let total_end = if line_end < src.len() {
                    line_end + 1
                } else {
                    line_end
                };
                self.cursor.pos = total_end;
                return Some(Block {
                    kind: BlockKind::Frontmatter,
                    span: Span::new(0, total_end),
                });
            }

            if line_end >= src.len() {
                return None; // no closing delimiter found
            }
            search_pos = line_end + 1;
        }
    }

    fn parse_block(&mut self) -> Block {
        let indent = self.cursor.leading_spaces();
        let line_after_indent = &self.cursor.rest_of_line()[indent..];

        match self.cursor.peek() {
            Some('#') if self.cursor.is_heading() => self.parse_heading(),
            Some(_) if Self::match_footnote_definition_marker(self.cursor.rest_of_line()).is_some() => {
                self.parse_footnote_definition()
            }
            Some(_) if Self::match_list_marker(line_after_indent).is_some() => {
                self.parse_list(indent)
            }
            Some(_) => self.parse_paragraph(),
            None => unreachable!("parse_block called at EOF; caller must check is_eof() first"),
        }
    }

    /// Recognizes a footnote definition marker (`[^id]:`) at the start of
    /// `line`. Returns the identifier's byte range within `line` and the
    /// byte length of the full marker (including the trailing `:`).
    fn match_footnote_definition_marker(line: &str) -> Option<(std::ops::Range<usize>, usize)> {
        let rest = line.strip_prefix("[^")?;
        let ident_len = rest.find(']')?;
        let ident = &rest[..ident_len];
        if ident.is_empty() || ident.chars().any(|c| c.is_whitespace()) {
            return None;
        }
        if !rest[ident_len..].starts_with("]:") {
            return None;
        }
        Some((2..2 + ident_len, 2 + ident_len + 2))
    }

    fn parse_footnote_definition(&mut self) -> Block {
        let start = self.cursor.pos;
        let (ident_range, marker_len) =
            Self::match_footnote_definition_marker(self.cursor.rest_of_line())
                .expect("caller verified marker presence");

        let identifier = Span::new(
            self.cursor.abs_pos() + ident_range.start,
            self.cursor.abs_pos() + ident_range.end,
        );
        for _ in 0..marker_len {
            self.cursor.next();
        }
        self.cursor.consume_while(|c| c == ' ');

        let inline_span = Span::new(
            self.cursor.abs_pos(),
            self.cursor.consume_while(|c| c != '\n').end,
        );
        let children = self.parse_inline(inline_span);

        self.cursor.consume_if(|c| c == '\n');

        Block {
            kind: BlockKind::FootnoteDefinition {
                identifier,
                children,
            },
            span: Span::new(start, self.cursor.pos),
        }
    }

    /// Recognizes a list marker at the start of `line` (with any leading
    /// indentation already stripped). Returns the list type and the byte
    /// length of the marker (including the trailing space).
    fn match_list_marker(line: &str) -> Option<(ListType, usize)> {
        for marker in ["- ", "* ", "+ "] {
            if line.starts_with(marker) {
                return Some((ListType::Unordered, marker.len()));
            }
        }

        let digits = line.chars().take_while(|c| c.is_ascii_digit()).count();
        if digits == 0 {
            return None;
        }
        let rest = &line[digits..];
        if rest.starts_with(". ") || rest.starts_with(") ") {
            return Some((ListType::Ordered, digits + 2));
        }

        None
    }

    fn parse_list(&mut self, indent: usize) -> Block {
        let start = self.cursor.pos;
        let mut items = Vec::new();
        let mut list_type = None;

        loop {
            if self.cursor.is_eof() {
                break;
            }

            let cur_indent = self.cursor.leading_spaces();
            if cur_indent != indent {
                break;
            }

            let line = &self.cursor.rest_of_line()[cur_indent..];
            let Some((kind, _)) = Self::match_list_marker(line) else {
                break;
            };

            if let Some(expected) = &list_type {
                if *expected != kind {
                    break;
                }
            } else {
                list_type = Some(kind);
            }

            items.push(self.parse_list_item(indent));
        }

        Block {
            kind: BlockKind::List {
                kind: list_type.unwrap_or(ListType::Unordered),
                children: items,
            },
            span: Span::new(start, self.cursor.pos),
        }
    }

    fn parse_list_item(&mut self, indent: usize) -> Block {
        let start = self.cursor.pos;

        self.cursor.consume_while(|c| c == ' '); // indentation
        let (_, marker_len) = Self::match_list_marker(self.cursor.rest_of_line())
            .expect("caller verified marker presence");
        for _ in 0..marker_len {
            self.cursor.next();
        }

        let checked = if self.cursor.starts_with("[ ] ") {
            for _ in 0.."[ ] ".len() {
                self.cursor.next();
            }
            Some(false)
        } else if self.cursor.starts_with("[x] ") || self.cursor.starts_with("[X] ") {
            for _ in 0.."[x] ".len() {
                self.cursor.next();
            }
            Some(true)
        } else {
            None
        };

        let inline_span = Span::new(
            self.cursor.abs_pos(),
            self.cursor.consume_while(|c| c != '\n').end,
        );
        let inline = self.parse_inline(inline_span);

        self.cursor.consume_if(|c| c == '\n');

        let mut children = Vec::new();
        if !self.cursor.is_eof() {
            let nested_indent = self.cursor.leading_spaces();
            let nested_line = &self.cursor.rest_of_line()[nested_indent..];
            if nested_indent > indent && Self::match_list_marker(nested_line).is_some() {
                children.push(self.parse_list(nested_indent));
            }
        }

        Block {
            kind: BlockKind::ListItem {
                checked,
                inline,
                children,
            },
            span: Span::new(start, self.cursor.pos),
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

            let indent = self.cursor.leading_spaces();
            let line_after_indent = &self.cursor.rest_of_line()[indent..];

            match next {
                None | Some('\n') => break,
                Some(_) if self.cursor.is_heading() => break,
                Some(_) if Self::match_list_marker(line_after_indent).is_some() => break,
                Some(_)
                    if Self::match_footnote_definition_marker(self.cursor.rest_of_line())
                        .is_some() =>
                {
                    break;
                }

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

        if cursor.starts_with("![") {
            let (alt_span, url_span, total_span) = Self::try_parse_image(&src[..span.end], abs)?;

            return Some((InlineKind::Image { alt_span, url_span }, total_span));
        }

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

        if cursor.starts_with("#") {
            let (name_span, total_span) = Self::try_parse_tag(src, abs)?;

            return Some((InlineKind::Tag { name: name_span }, total_span));
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

    /// Try to extract an image (`![alt](url)`).
    ///
    /// # Returns
    ///
    /// `Some((alt_span, url_span, total_span))` on success, `None` otherwise.
    fn try_parse_image(src: &str, pos: usize) -> Option<(Span, Span, Span)> {
        if !src[pos..].starts_with('!') {
            return None;
        }
        let (alt_span, url_span, link_span) = Self::try_parse_link(src, pos + 1)?;
        Some((alt_span, url_span, Span::new(pos, link_span.end)))
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
        let rest = src.get(ident_start..)?;
        let ident_end = rest.find(']').map(|i| ident_start + i)?;

        if ident_start == ident_end {
            return None;
        }

        if src
            .get(ident_start..ident_end)?
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

    /// Try to extract a `#tag` at `pos`.
    ///
    /// # Returns
    ///
    /// `Some((name_span, total_span))` where `name_span` excludes the
    /// leading `#`. Returns `None` when no valid tag is found.
    fn try_parse_tag(src: &str, pos: usize) -> Option<(Span, Span)> {
        let rest = &src[pos + 1..];
        let first = rest.chars().next()?;
        if !(first.is_alphanumeric() || first == '_') {
            return None;
        }

        let len: usize = rest
            .chars()
            .take_while(|&c| c.is_alphanumeric() || c == '_' || c == '-' || c == '/')
            .map(|c| c.len_utf8())
            .sum();

        let name_span = Span::new(pos + 1, pos + 1 + len);
        Some((name_span, Span::new(pos, pos + 1 + len)))
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
                children: vec![
                    Inline {
                        kind: InlineKind::Text,
                        span: Span::new(0, 4),
                    },
                    Inline {
                        kind: InlineKind::Footnote {
                            identifier: Span::new(6, 9),
                        },
                        span: Span::new(4, 10),
                    },
                    Inline {
                        kind: InlineKind::Text,
                        span: Span::new(10, 15),
                    },
                ],
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
    fn parse_frontmatter() {
        let input = "---\ntitle: Hello\n---\n# Heading";
        let result = Parser::new(input).parse();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].kind, BlockKind::Frontmatter);
        assert_eq!(result[0].span, Span::new(0, 21));
        assert!(matches!(result[1].kind, BlockKind::Heading { .. }));
    }

    #[test]
    fn frontmatter_only_recognized_at_start() {
        let input = "text\n---\ntitle: Hello\n---\n";
        let result = Parser::new(input).parse();

        assert!(
            result
                .iter()
                .all(|b| !matches!(b.kind, BlockKind::Frontmatter))
        );
    }

    #[test]
    fn unclosed_frontmatter_is_not_frontmatter() {
        let input = "---\ntitle: Hello\n";
        let result = Parser::new(input).parse();

        assert!(
            result
                .iter()
                .all(|b| !matches!(b.kind, BlockKind::Frontmatter))
        );
    }

    #[test]
    fn parse_tag() {
        let input = "hello #project world";
        let result = Parser::new(input).parse();

        let BlockKind::Paragraph { children } = &result[0].kind else {
            panic!("expected Paragraph");
        };
        assert_eq!(children.len(), 3);
        let InlineKind::Tag { name } = &children[1].kind else {
            panic!("expected Tag");
        };
        assert_eq!(name.as_str(input), "project");
        assert_eq!(children[1].span.as_str(input), "#project");
    }

    #[test]
    fn parse_tag_with_slash_and_dash() {
        let input = "#area/sub-topic more";
        let result = Parser::new(input).parse();

        let BlockKind::Paragraph { children } = &result[0].kind else {
            panic!("expected Paragraph");
        };
        let InlineKind::Tag { name } = &children[0].kind else {
            panic!("expected Tag");
        };
        assert_eq!(name.as_str(input), "area/sub-topic");
    }

    #[test]
    fn heading_marker_is_not_a_tag() {
        let input = "# Heading";
        let result = Parser::new(input).parse();
        assert!(matches!(result[0].kind, BlockKind::Heading { .. }));
    }

    #[test]
    fn bare_hash_without_word_char_is_text() {
        let input = "just a # sign";
        let result = Parser::new(input).parse();

        let BlockKind::Paragraph { children } = &result[0].kind else {
            panic!("expected Paragraph");
        };
        assert_eq!(children.len(), 1);
        assert!(matches!(children[0].kind, InlineKind::Text));
    }

    #[test]
    fn parse_image() {
        let input = "![alt text](image.png)";
        let result = Parser::new(input).parse();

        let expected = vec![Block {
            span: Span::new(0, 22),
            kind: BlockKind::Paragraph {
                children: vec![Inline {
                    span: Span::new(0, 22),
                    kind: InlineKind::Image {
                        alt_span: Span::new(2, 10),
                        url_span: Span::new(12, 21),
                    },
                }],
            },
        }];

        assert_eq!(result, expected);
    }

    #[test]
    fn parse_image_mid_string() {
        let input = "see ![alt](img.png) here";
        let result = Parser::new(input).parse();

        let BlockKind::Paragraph { children } = &result[0].kind else {
            panic!("expected Paragraph");
        };
        assert_eq!(children.len(), 3);
        assert!(matches!(children[1].kind, InlineKind::Image { .. }));
        assert_eq!(children[1].span.as_str(input), "![alt](img.png)");
    }

    #[test]
    fn image_does_not_conflict_with_link() {
        let input = "[text](url) ![alt](img.png)";
        let result = Parser::new(input).parse();

        let BlockKind::Paragraph { children } = &result[0].kind else {
            panic!("expected Paragraph");
        };
        assert!(matches!(children[0].kind, InlineKind::Link { .. }));
        assert!(matches!(children[2].kind, InlineKind::Image { .. }));
    }

    #[test]
    fn try_parse_image_basic() {
        let input = "![alt](img.png)";
        let result = Parser::try_parse_image(input, 0);

        assert_eq!(
            result,
            Some((Span::new(2, 5), Span::new(7, 14), Span::new(0, 15)))
        );
    }

    #[test]
    fn parse_footnote_definition() {
        let input = "[^note]: this is the note";
        let result = Parser::new(input).parse();

        assert_eq!(result.len(), 1);
        let BlockKind::FootnoteDefinition {
            identifier,
            children,
        } = &result[0].kind
        else {
            panic!("expected FootnoteDefinition");
        };
        assert_eq!(identifier.as_str(input), "note");
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].span.as_str(input), "this is the note");
    }

    #[test]
    fn parse_footnote_definition_rejects_whitespace_identifier() {
        let input = "[^a b]: not a definition";
        let result = Parser::new(input).parse();
        assert!(matches!(result[0].kind, BlockKind::Paragraph { .. }));
    }

    #[test]
    fn parse_footnote_definition_after_paragraph() {
        let input = "See[^note] below\n\n[^note]: the note text";
        let result = Parser::new(input).parse();

        assert_eq!(result.len(), 2);
        assert!(matches!(result[0].kind, BlockKind::Paragraph { .. }));
        assert!(matches!(
            result[1].kind,
            BlockKind::FootnoteDefinition { .. }
        ));
    }

    #[test]
    fn parse_unordered_list() {
        let input = "- one\n- two\n- three";
        let result = Parser::new(input).parse();

        assert_eq!(result.len(), 1);
        let BlockKind::List { kind, children } = &result[0].kind else {
            panic!("expected List");
        };
        assert_eq!(*kind, ListType::Unordered);
        assert_eq!(children.len(), 3);

        for (item, text) in children.iter().zip(["one", "two", "three"]) {
            let BlockKind::ListItem {
                checked, inline, ..
            } = &item.kind
            else {
                panic!("expected ListItem");
            };
            assert!(checked.is_none());
            assert_eq!(inline.len(), 1);
            assert!(matches!(inline[0].kind, InlineKind::Text));
            assert_eq!(inline[0].span.as_str(input), text);
        }
    }

    #[test]
    fn parse_ordered_list() {
        let input = "1. one\n2. two\n3. three";
        let result = Parser::new(input).parse();

        let BlockKind::List { kind, children } = &result[0].kind else {
            panic!("expected List");
        };
        assert_eq!(*kind, ListType::Ordered);
        assert_eq!(children.len(), 3);
    }

    #[test]
    fn parse_list_item_markers() {
        for marker in ["-", "*", "+"] {
            let input = format!("{marker} item");
            let result = Parser::new(&input).parse();
            let BlockKind::List { kind, .. } = &result[0].kind else {
                panic!("expected List for marker {marker}");
            };
            assert_eq!(*kind, ListType::Unordered);
        }
    }

    #[test]
    fn parse_list_with_checkboxes() {
        let input = "- [ ] todo\n- [x] done\n- [X] also done\n- plain";
        let result = Parser::new(input).parse();

        let BlockKind::List { children, .. } = &result[0].kind else {
            panic!("expected List");
        };
        assert_eq!(children.len(), 4);

        let expect = [Some(false), Some(true), Some(true), None];
        for (item, checked) in children.iter().zip(expect) {
            let BlockKind::ListItem { checked: c, .. } = &item.kind else {
                panic!("expected ListItem");
            };
            assert_eq!(*c, checked);
        }
    }

    #[test]
    fn parse_nested_list() {
        let input = "- outer\n  - inner one\n  - inner two\n- outer two";
        let result = Parser::new(input).parse();

        let BlockKind::List { children, .. } = &result[0].kind else {
            panic!("expected List");
        };
        assert_eq!(children.len(), 2);

        let BlockKind::ListItem {
            children: nested, ..
        } = &children[0].kind
        else {
            panic!("expected ListItem");
        };
        assert_eq!(nested.len(), 1);
        let BlockKind::List {
            children: nested_items,
            ..
        } = &nested[0].kind
        else {
            panic!("expected nested List");
        };
        assert_eq!(nested_items.len(), 2);
    }

    #[test]
    fn parse_list_with_inline_formatting() {
        let input = "- **bold** item";
        let result = Parser::new(input).parse();

        let BlockKind::List { children, .. } = &result[0].kind else {
            panic!("expected List");
        };
        let BlockKind::ListItem { inline, .. } = &children[0].kind else {
            panic!("expected ListItem");
        };
        assert!(matches!(inline[0].kind, InlineKind::Bold { .. }));
    }

    #[test]
    fn parse_list_after_paragraph() {
        let input = "hello world\n- one\n- two";
        let result = Parser::new(input).parse();

        assert_eq!(result.len(), 2);
        assert!(matches!(result[0].kind, BlockKind::Paragraph { .. }));
        assert!(matches!(result[1].kind, BlockKind::List { .. }));
    }

    #[test]
    fn text_starting_with_digit_is_paragraph_not_list() {
        let input = "123 not a list";
        let result = Parser::new(input).parse();
        assert!(matches!(result[0].kind, BlockKind::Paragraph { .. }));
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
