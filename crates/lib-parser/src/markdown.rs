use chumsky::prelude::*;

use crate::{InlineMarkdownNode, LinkType, MarkdownNode, ParseError, Spanned};

pub fn header_parser<'a>() -> impl Parser<'a, &'a str, MarkdownNode<'a>, ParseError<'a>> {
    let hashes = just('#')
        .repeated()
        .at_least(1)
        .at_most(6)
        .count()
        .labelled("hashes");

    let line_text = any()
        .filter(|c: &char| *c != '\n' && *c != '\r')
        .repeated()
        .to_slice()
        .map(|s: &'a str| s.trim_end().trim_end_matches('#').trim())
        .labelled("header text");

    let required_space = any()
        .filter(|c: &char| *c == ' ' || *c == '\t')
        .repeated()
        .at_least(1)
        .ignored()
        .labelled("space after #");

    hashes
        .then_ignore(required_space)
        .then(line_text)
        .map(|(hashes, text_slice)| MarkdownNode::Header {
            level: hashes,
            content: text_slice,
        })
        .labelled("Header Parser")
}

pub fn tag_parser<'a>() -> impl Parser<'a, &'a str, InlineMarkdownNode<'a>, ParseError<'a>> {
    just('#')
        .ignore_then(
            any()
                .filter(|c: &char| c.is_alphanumeric() || *c == '-' || *c == '_')
                .repeated()
                .at_least(1)
                .to_slice(),
        )
        .map(InlineMarkdownNode::Tag)
        .labelled("Tag Parser")
}

pub fn footnote_parser<'a>() -> impl Parser<'a, &'a str, InlineMarkdownNode<'a>, ParseError<'a>> {
    just("[^")
        .ignore_then(
            any()
                .filter(|c: &char| c.is_alphanumeric() || *c == '-' || *c == '_')
                .repeated()
                .at_least(1)
                .to_slice(),
        )
        .then_ignore(just("]"))
        .map(InlineMarkdownNode::Footnote)
        .labelled("Footnote Parser")
}

pub fn footnote_definition_parser<'a>() -> impl Parser<'a, &'a str, MarkdownNode<'a>, ParseError<'a>>
{
    let id = just("[^")
        .ignore_then(
            any()
                .filter(|c: &char| c.is_alphanumeric() || *c == '-' || *c == '_')
                .repeated()
                .at_least(1)
                .to_slice(),
        )
        .then_ignore(just("]"))
        .then_ignore(just(":"))
        .labelled("Footnote Def Id");

    let inline_text = inline_parser()
        .map_with(|block, e| Spanned(block, e.span()))
        .repeated()
        .at_least(1)
        .collect();

    id.then(inline_text)
        .then_ignore(text::newline().or(end()))
        .map(|(id, content)| MarkdownNode::FootnoteDefinition { id, content })
        .labelled("Footnote Definition Parser")
}

pub fn wikilink_parser<'a>() -> impl Parser<'a, &'a str, InlineMarkdownNode<'a>, ParseError<'a>> {
    let alias = any()
        .filter(|c: &char| *c != ']' && *c != '\n')
        .repeated()
        .to_slice()
        .map(|alias: &'a str| alias.trim())
        .map(|alias| (!alias.is_empty()).then_some(alias));

    let header_content = any()
        .filter(|c: &char| !['|', ']', '\n'].contains(c))
        .repeated()
        .at_least(1)
        .to_slice()
        .labelled("WikiLink Header Parser");

    let header = just('#')
        .ignore_then(header_content)
        .map(|content| content)
        .labelled("Header Level Parser");

    let possible_alias = choice((
        just('|').ignore_then(alias).then_ignore(just("]]")),
        just("]]").to(None),
    ));

    let target = any()
        .filter(|c: &char| !['#', ']', '|', '\n'].contains(c))
        .repeated()
        .at_least(1)
        .to_slice()
        .map(|s: &'a str| s.trim())
        .then(header.or_not())
        .then(possible_alias);

    just("[[")
        .ignore_then(target)
        .map_err(|e: Rich<char>| Rich::custom(*e.span(), "WikiLink format is invalid."))
        .map(|((target, header), display_text)| {
            InlineMarkdownNode::Link(LinkType::WikiLink {
                target,
                display_text,
                header,
            })
        })
        .labelled("WikiLink")
}

pub fn link_parser<'a>() -> impl Parser<'a, &'a str, InlineMarkdownNode<'a>, ParseError<'a>> {
    let title = any()
        .filter(|c: &char| *c != ']' && *c != '\n')
        .repeated()
        .at_least(1)
        .to_slice()
        .map(|title: &str| title.trim())
        .labelled("Link Title Parser");

    let header_content = any()
        .filter(|c: &char| ![')', '\n'].contains(c))
        .repeated()
        .at_least(1)
        .to_slice()
        .labelled("WikiLink Header Parser");

    let header = just('#')
        .ignore_then(header_content)
        .map(|content| content)
        .labelled("Header Level Parser");

    let uri = any()
        .filter(|c: &char| !['#', ')', '\n'].contains(c))
        .repeated()
        .to_slice()
        .map(|uri: &str| uri.trim())
        .then(header.or_not())
        .labelled("Link URL Parser");

    just('[')
        .ignore_then(title)
        .then_ignore(just(']'))
        // TODO: Build out these errors for for information
        .map_err(|e: Rich<char>| Rich::custom(*e.span(), "1. Link format is invalid."))
        .then_ignore(just('('))
        .then(uri)
        .map_err(|e: Rich<char>| Rich::custom(*e.span(), "2. Link format is invalid."))
        .then_ignore(just(')'))
        .map(|(text, (uri, header))| {
            InlineMarkdownNode::Link(LinkType::InlineLink { text, uri, header })
        })
        .labelled("Link Parser")
}

pub fn image_parser<'a>() -> impl Parser<'a, &'a str, InlineMarkdownNode<'a>, ParseError<'a>> {
    let alt = any()
        .filter(|c: &char| *c != ']' && *c != '\n')
        .repeated()
        .to_slice();

    let uri = any()
        .filter(|c: &char| *c != ')' && *c != '\n')
        .repeated()
        .at_least(1)
        .to_slice();

    just("![")
        .ignore_then(alt)
        .then_ignore(just("]("))
        .then(uri)
        .then_ignore(just(')'))
        .map(|(text, uri)| InlineMarkdownNode::Link(LinkType::ImageLink { text, uri }))
        .labelled("Image")
}

pub fn plain_text_parser<'a>() -> impl Parser<'a, &'a str, InlineMarkdownNode<'a>, ParseError<'a>> {
    let stop_condition = choice((
        just("#"),
        just("["),
        just("!["),
        just("\n\n"),
    ))
    .rewind();

    any()
        .and_is(stop_condition.not())
        .repeated()
        .at_least(1)
        .to_slice()
        .map(InlineMarkdownNode::PlainText)
        .labelled("Plain Text")
}

// Line-bounded plain text parser for use in list items (stops at single newline)
pub fn line_plain_text_parser<'a>()
-> impl Parser<'a, &'a str, InlineMarkdownNode<'a>, ParseError<'a>> {
    let stop_condition = choice((
        just("#"),
        just("["),
        just("!["),
        just("\n"),
    ))
    .rewind();

    any()
        .and_is(stop_condition.not())
        .repeated()
        .at_least(1)
        .to_slice()
        .map(InlineMarkdownNode::PlainText)
        .labelled("Plain Text")
}

// Line-bounded inline parser for use in list items
pub fn line_inline_parser<'a>() -> impl Parser<'a, &'a str, InlineMarkdownNode<'a>, ParseError<'a>>
{
    choice((
        tag_parser(),
        image_parser(),
        wikilink_parser(),
        footnote_parser(),
        link_parser(),
        line_plain_text_parser(),
    ))
    .labelled("Line Inline Parser")
}

pub fn inline_parser<'a>() -> impl Parser<'a, &'a str, InlineMarkdownNode<'a>, ParseError<'a>> {
    choice((
        tag_parser(),
        image_parser(),
        wikilink_parser(),
        link_parser(),
        footnote_parser(),
        plain_text_parser(),
    ))
    .labelled("Inline Parser")
}

pub fn list_item_parser<'a>() -> impl Parser<'a, &'a str, MarkdownNode<'a>, ParseError<'a>> {
    let marker = choice((just('-'), just('*')))
        .then_ignore(text::inline_whitespace())
        .labelled("list marker");

    let checkbox = just('[')
        .ignore_then(choice((
            just(' ').to(Some(false)),
            just('x').to(Some(true)),
            just('X').to(Some(true)),
        )))
        .then_ignore(just(']'))
        .then_ignore(text::inline_whitespace())
        .or_not()
        .labelled("checkbox");

    // Use line_inline_parser to prevent consuming across newlines
    let content = line_inline_parser()
        .map_with(|inline_block, e| Spanned(inline_block, e.span()))
        .repeated()
        .at_least(1)
        .collect();

    marker
        .ignore_then(checkbox)
        .then(content)
        .map(|(checkbox, content)| MarkdownNode::ListItem {
            checkbox: checkbox.flatten(),
            content,
        })
        .labelled("List Item")
}

pub fn paragraph_parser<'a>() -> impl Parser<'a, &'a str, MarkdownNode<'a>, ParseError<'a>> {
    inline_parser()
        .map_with(|inline_block, e| Spanned(inline_block, e.span()))
        .repeated()
        .at_least(1)
        .collect()
        .map(MarkdownNode::Paragraph)
}
