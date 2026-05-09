use lib_parser::new_markdown::Span;

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
        alias: Option<Span>,
    },
    Inline {
        label: Span,
        url: Span,
        title: Option<Span>,
    },
}

#[derive(Debug, Clone)]
pub struct Link {
    pub span: Span,
    pub kind: LinkKind,
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
