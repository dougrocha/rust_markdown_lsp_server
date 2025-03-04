pub mod markdown_parser;

use chumsky::span::SimpleSpan;

pub use chumsky::Parser;

pub type MarkdownText<'a> = Vec<Spanned<InlineMarkdown<'a>>>;

#[derive(Debug, Clone, PartialEq)]
pub enum Markdown<'a> {
    Header {
        level: usize,
        content: &'a str,
    },
    Paragraph(MarkdownText<'a>),
    FootnoteDefinition {
        id: &'a str,
        content: MarkdownText<'a>,
    },
    Invalid,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InlineMarkdown<'a> {
    PlainText(&'a str),
    Link {
        title: &'a str,
        url: &'a str,
        // Used for links to other markdown files with a specific header
        header: Option<&'a str>,
    },
    Image {
        alt_text: &'a str,
        uri: &'a str,
    },
    WikiLink {
        target: &'a str,
        alias: Option<&'a str>,
        header: Option<&'a str>,
    },
    Footnote(&'a str),
    Tag(&'a str),
    Invalid,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Spanned<T>(pub T, pub SimpleSpan);
