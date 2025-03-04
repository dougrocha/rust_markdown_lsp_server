use chumsky::prelude::*;

use crate::{InlineMarkdown, Markdown, Spanned};

pub fn header_parser<'a>() -> impl Parser<'a, &'a str, Markdown<'a>, extra::Err<Rich<'a, char>>> {
    let hashes = just('#')
        .repeated()
        .at_least(1)
        .at_most(6)
        .count()
        .labelled("hashes");

    let line_text = any()
        .filter(|x: &char| *x != '\n')
        .repeated()
        .to_slice()
        .labelled("header text");

    hashes
        .padded()
        .then(line_text)
        .then_ignore(text::newline())
        .map(|(hashes, text)| Markdown::Header {
            level: hashes,
            content: text,
        })
        .labelled("Header Parser")
}

pub fn tag_parser<'a>() -> impl Parser<'a, &'a str, InlineMarkdown<'a>, extra::Err<Rich<'a, char>>>
{
    just('#')
        .ignore_then(
            any()
                .filter(|c: &char| c.is_alphanumeric())
                .repeated()
                .at_least(1)
                .to_slice(),
        )
        .map(InlineMarkdown::Tag)
        .labelled("Tag Parser")
}

pub fn footnote_parser<'a>(
) -> impl Parser<'a, &'a str, InlineMarkdown<'a>, extra::Err<Rich<'a, char>>> {
    just("[^")
        .ignore_then(
            any()
                .filter(|c: &char| c.is_alphanumeric())
                .repeated()
                .at_least(1)
                .to_slice(),
        )
        .then_ignore(just("]"))
        .map(InlineMarkdown::Footnote)
        .labelled("Footnote Parser")
}

pub fn footnote_definition_parser<'a>(
) -> impl Parser<'a, &'a str, Markdown<'a>, extra::Err<Rich<'a, char>>> {
    let id = just("[^")
        .ignore_then(
            any()
                .filter(|c: &char| c.is_alphanumeric())
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
        .map(|(id, content)| Markdown::FootnoteDefinition { id, content })
        .labelled("Footnote Definition Parser")
}

pub fn wikilink_parser<'a>(
) -> impl Parser<'a, &'a str, InlineMarkdown<'a>, extra::Err<Rich<'a, char>>> {
    let alias = any()
        .filter(|c: &char| *c != ']' && *c != '\n')
        .repeated()
        .to_slice()
        .validate(|x: &str, e, emitter| {
            if x.starts_with(" ") || x.ends_with(" ") {
                emitter.emit(Rich::custom(
                    e.span(),
                    "WikiLink alias contains spaces before or after.",
                ));
            }

            x
        })
        .map(|alias: &'a str| alias.trim())
        .map(|alias| (!alias.is_empty()).then_some(alias));

    let header = just('#').rewind().ignore_then(
        any()
            .filter(|c: &char| !['|', ']', '\n'].contains(c))
            .repeated()
            .at_least(1)
            .to_slice()
            .labelled("WikiLink Header Parser"),
    );

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
        .map(|((target, header), alias)| InlineMarkdown::WikiLink {
            target,
            alias,
            header,
        })
        .labelled("WikiLink")
}

pub fn link_parser<'a>() -> impl Parser<'a, &'a str, InlineMarkdown<'a>, extra::Err<Rich<'a, char>>>
{
    let title = any()
        .filter(|c: &char| *c != ']' && *c != '\n')
        .repeated()
        .at_least(1)
        .to_slice()
        .validate(|x: &str, e, emitter| {
            if x.starts_with(" ") || x.ends_with(" ") {
                emitter.emit(Rich::custom(
                    e.span(),
                    "Link title contains spaces before or after",
                ));
            }

            x
        })
        .map(|title: &str| title.trim())
        .labelled("Link Title Parser");

    let header = just('#').ignore_then(
        any()
            .filter(|c: &char| *c != ']' && *c != '\n')
            .repeated()
            .at_least(1)
            .to_slice()
            .map(|title: &str| title.trim())
            .labelled("Link Header Parser"),
    );

    let uri = any()
        .filter(|c: &char| *c != '#' && *c != ')' && *c != '\n')
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
        .map(|(title, (url, header))| InlineMarkdown::Link { title, url, header })
        .labelled("Link Parser")
}

pub fn image_parser<'a>() -> impl Parser<'a, &'a str, InlineMarkdown<'a>, extra::Err<Rich<'a, char>>>
{
    let alt_text = any()
        .filter(|c: &char| *c != ']' && *c != '\n')
        .repeated()
        .at_least(1)
        .to_slice()
        .map(|alias: &'a str| alias.trim())
        .labelled("Alt Text Parser");

    // TODO: Add this to diagnostic errors
    let uri = any()
        .filter(|c: &char| *c != ')' && *c != '\n')
        .repeated()
        .to_slice()
        .map(|alias: &'a str| alias.trim())
        .labelled("Image URI Parser");

    just("![")
        .ignore_then(alt_text)
        .then_ignore(just(']'))
        .then(just('(').ignore_then(uri).then_ignore(just(')')))
        .map(|(alt_text, uri)| InlineMarkdown::Image { alt_text, uri })
        .labelled("Image Parser")
}

pub fn plain_text_parser<'a>(
) -> impl Parser<'a, &'a str, InlineMarkdown<'a>, extra::Err<Rich<'a, char>>> {
    let stop_condition = choice((
        just("#"),
        just("["),
        just("[^"),
        just("[["),
        just("!["),
        just("\n\n"),
    ))
    .rewind();

    any()
        .and_is(stop_condition.not())
        .repeated()
        .at_least(1)
        .to_slice()
        .map(InlineMarkdown::PlainText)
        .labelled("Plain Text")
}

pub fn inline_parser<'a>(
) -> impl Parser<'a, &'a str, InlineMarkdown<'a>, extra::Err<Rich<'a, char>>> {
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

pub fn paragraph_parser<'a>() -> impl Parser<'a, &'a str, Markdown<'a>, extra::Err<Rich<'a, char>>>
{
    inline_parser()
        .map_with(|inline_block, e| Spanned(inline_block, e.span()))
        .repeated()
        .at_least(1)
        .collect()
        .map(Markdown::Paragraph)
}

pub fn markdown_parser<'a>(
) -> impl Parser<'a, &'a str, Vec<Spanned<Markdown<'a>>>, extra::Err<Rich<'a, char>>> {
    choice((
        header_parser(),
        footnote_definition_parser(),
        paragraph_parser(),
    ))
    .recover_with(skip_until(
        any().ignored(),
        text::newline().ignored(),
        || Markdown::Invalid,
    ))
    .map_with(|block, e| Spanned(block, e.span()))
    .then_ignore(choice((text::whitespace(), text::newline())))
    .repeated()
    .collect()
    .then_ignore(end().or_not())
}
