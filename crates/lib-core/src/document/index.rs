use std::borrow::Cow;

use lib_parser::markdown::Span;
use ropey::Rope;

#[derive(Debug, Clone)]
pub struct Header {
    pub span: Span,
    pub text_span: Span,
    pub level: u8,
}

impl Header {
    /// Gets the header's text content from source
    ///
    /// Returns Cow, as memory may not be together in Rope
    pub fn content_str<'a>(&self, source: &'a Rope) -> Cow<'a, str> {
        source
            .byte_slice(self.text_span.start..self.text_span.end)
            .into()
    }
}

#[derive(Debug, Clone)]
pub struct Tag {
    pub span: Span,
    pub name_span: Span,
}

impl Tag {
    /// Gets the tag's name (without the leading `#`) from source
    ///
    /// Returns Cow, as memory may not be together in Rope
    pub fn name_str<'a>(&self, source: &'a Rope) -> Cow<'a, str> {
        source
            .byte_slice(self.name_span.start..self.name_span.end)
            .into()
    }
}

#[derive(Debug, Clone)]
pub enum LinkKind {
    Wiki {
        target: Span,
        header: Option<Span>,
        alias: Option<Span>,
    },
    Inline {
        label: Span,
        url: Span,
        header: Option<Span>,
        title: Option<Span>,
    },
    Image {
        alt: Span,
        url: Span,
    },
}

#[derive(Debug, Clone)]
pub struct Link {
    pub span: Span,
    pub kind: LinkKind,
}

impl Link {
    pub fn header(&self) -> Option<Span> {
        match &self.kind {
            LinkKind::Wiki { header, .. } => *header,
            LinkKind::Inline { header, .. } => *header,
            LinkKind::Image { .. } => None,
        }
    }

    /// Gets the target string from source
    ///
    /// Returns Cow, as memory may not be together in Rope
    pub fn target_str<'a>(&self, source: &'a Rope) -> Cow<'a, str> {
        match &self.kind {
            LinkKind::Wiki { target, .. } => source.byte_slice(target.start..target.end).into(),
            LinkKind::Inline { url, .. } => source.byte_slice(url.start..url.end).into(),
            LinkKind::Image { url, .. } => source.byte_slice(url.start..url.end).into(),
        }
    }

    /// Gets the header string from source
    ///
    /// Returns Cow, as memory may not be together in Rope
    pub fn header_str<'a>(&self, source: &'a Rope) -> Option<Cow<'a, str>> {
        self.header()
            .map(|s| source.byte_slice(s.start..s.end).into())
    }

    /// Gets the wikilink alias string from source, if any
    ///
    /// Returns Cow, as memory may not be together in Rope
    pub fn alias_str<'a>(&self, source: &'a Rope) -> Option<Cow<'a, str>> {
        match &self.kind {
            LinkKind::Wiki { alias, .. } => {
                alias.map(|s| source.byte_slice(s.start..s.end).into())
            }
            LinkKind::Inline { .. } | LinkKind::Image { .. } => None,
        }
    }

    /// Gets the inline link's label string from source, if any
    ///
    /// Returns Cow, as memory may not be together in Rope
    pub fn label_str<'a>(&self, source: &'a Rope) -> Option<Cow<'a, str>> {
        match &self.kind {
            LinkKind::Inline { label, .. } => {
                Some(source.byte_slice(label.start..label.end).into())
            }
            LinkKind::Image { alt, .. } => Some(source.byte_slice(alt.start..alt.end).into()),
            LinkKind::Wiki { .. } => None,
        }
    }

    /// Renders this link's markdown text with its target replaced, preserving
    /// the original syntax (wikilink vs inline), header fragment, and alias/label.
    pub fn render_with_target(&self, source: &Rope, new_target: &str) -> String {
        let mut path = new_target.to_string();
        if let Some(header) = self.header_str(source) {
            path.push('#');
            path.push_str(&header);
        }

        match &self.kind {
            LinkKind::Wiki { .. } => match self.alias_str(source) {
                Some(alias) => format!("[[{path}|{alias}]]"),
                None => format!("[[{path}]]"),
            },
            LinkKind::Inline { .. } => {
                let label = self.label_str(source).unwrap_or_default();
                format!("[{label}]({path})")
            }
            LinkKind::Image { .. } => {
                let alt = self.label_str(source).unwrap_or_default();
                format!("![{alt}]({path})")
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct FootnoteDefinition {
    pub span: Span,
    pub identifier: Span,
    pub content_span: Span,
}

impl FootnoteDefinition {
    /// Gets the footnote identifier's text content from source
    ///
    /// Returns Cow, as memory may not be together in Rope
    pub fn identifier_str<'a>(&self, source: &'a Rope) -> Cow<'a, str> {
        source
            .byte_slice(self.identifier.start..self.identifier.end)
            .into()
    }

    /// Gets the footnote's content text from source
    ///
    /// Returns Cow, as memory may not be together in Rope
    pub fn content_str<'a>(&self, source: &'a Rope) -> Cow<'a, str> {
        source
            .byte_slice(self.content_span.start..self.content_span.end)
            .into()
    }
}

#[derive(Debug, Clone)]
pub struct FootnoteReference {
    pub span: Span,
    pub identifier: Span,
}

impl FootnoteReference {
    /// Gets the footnote identifier's text content from source
    ///
    /// Returns Cow, as memory may not be together in Rope
    pub fn identifier_str<'a>(&self, source: &'a Rope) -> Cow<'a, str> {
        source
            .byte_slice(self.identifier.start..self.identifier.end)
            .into()
    }
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub span: Span,
    pub severity: Severity,
    pub code: DiagnosticCode,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DiagnosticCode {
    BrokenWikilink,
    BrokenInlineLink,
    MissingFrontmatterField,
    MalformedTag,
}
