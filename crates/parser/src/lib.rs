use chumsky::prelude::*;

use markdown::{footnote_definition_parser, header_parser, paragraph_parser};
use yaml::{frontmatter_parser, Frontmatter};

pub use chumsky::Parser;

pub mod markdown;
pub mod yaml;

#[derive(Debug, Clone, PartialEq)]
pub struct Spanned<T>(pub T, pub SimpleSpan);

pub type ParseError<'a> = extra::Err<Rich<'a, char>>;

pub type MarkdownText<'a> = Vec<Spanned<InlineMarkdownNode<'a>>>;

#[derive(Debug, Clone)]
pub struct ParsedMarkdown<'a> {
    pub frontmatter: Option<Frontmatter<'a>>,
    pub body: Vec<Spanned<MarkdownNode<'a>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HeaderRef<'a> {
    /// TODO: Remove this eventually
    pub level: usize,
    pub slug: &'a str,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MarkdownNode<'a> {
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
pub enum LinkType<'a> {
    InlineLink {
        text: &'a str,
        uri: &'a str,
        header: Option<&'a str>,
    },
    WikiLink {
        target: &'a str,
        display_text: Option<&'a str>,
        header: Option<&'a str>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum InlineMarkdownNode<'a> {
    PlainText(&'a str),
    Link(LinkType<'a>),
    Tag(&'a str),
    Footnote(&'a str),
    Invalid,
}

pub fn markdown_parser<'a>() -> impl Parser<'a, &'a str, ParsedMarkdown<'a>, ParseError<'a>> {
    frontmatter_parser()
        .or_not()
        .then(
            choice((
                header_parser(),
                footnote_definition_parser(),
                paragraph_parser(),
            ))
            .recover_with(skip_until(
                any().ignored(),
                text::newline().ignored(),
                || MarkdownNode::Invalid,
            ))
            .map_with(|block, e| Spanned(block, e.span()))
            .then_ignore(choice((text::whitespace(), text::newline())))
            .repeated()
            .collect(),
        )
        .then_ignore(end().or_not())
        .map(|(frontmatter, body)| ParsedMarkdown { frontmatter, body })
}
