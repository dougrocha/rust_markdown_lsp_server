use std::borrow::Cow;

use lib_parser::new_markdown::Span;
use ropey::Rope;

#[derive(Debug, Clone)]
pub struct Header {
    pub span: Span,
    pub text_span: Span,
    pub level: u8,
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
        }
    }

    /// Gets the target string from source
    ///
    /// Returns Cow, as memory may not be together in Rope
    pub fn target_str<'a>(&self, source: &'a Rope) -> Cow<'a, str> {
        match &self.kind {
            LinkKind::Wiki { target, .. } => source.byte_slice(target.start..target.end).into(),
            LinkKind::Inline { url, .. } => source.byte_slice(url.start..url.end).into(),
        }
    }

    /// Gets the header string from source
    ///
    /// Returns Cow, as memory may not be together in Rope
    pub fn header_str<'a>(&self, source: &'a Rope) -> Option<Cow<'a, str>> {
        self.header()
            .map(|s| source.byte_slice(s.start..s.end).into())
    }
}

pub struct Diagnostic {
    pub span: Span,
    pub severity: Severity,
    pub code: DiagnosticCode,
}

pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}

pub enum DiagnosticCode {
    BrokenWikilink,
    BrokenInlineLink,
    MissingFrontmatterField,
}
