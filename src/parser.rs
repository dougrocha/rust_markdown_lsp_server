use std::ops::Range;

use chumsky::prelude::*;

type MarkdownText = Vec<InlineMarkdown>;

#[derive(Debug, Clone)]
pub enum Markdown {
    Header { level: usize, content: MarkdownText },
    Paragraph(MarkdownText),
    ReferenceDefinition { id: String, content: MarkdownText },
    CodeBlock(String),
    Error,
}

#[derive(Debug, Clone)]
pub enum InlineMarkdown {
    PlainText(String),
    Link {
        text: String,
        url: String,
    },
    Image {
        alt_text: String,
        url: String,
    },
    WikiLink {
        target: String,
        alias: Option<String>,
    },
    Reference(String),
}

#[derive(Debug, Clone)]
pub struct SpannedMarkdown(pub Markdown, pub Range<usize>);

pub fn header_parser() -> impl Parser<char, Markdown, Error = Simple<char>> {
    println!("Parsing Header");
    just('#')
        .repeated()
        .at_least(1)
        .at_most(6)
        .then_ignore(just(' '))
        .then(inline_parser().repeated())
        .then_ignore(text::newline())
        .map_err_with_span(|_err, span| Simple::custom(span, "Malformed Header"))
        .map(|(hashes, inlines)| Markdown::Header {
            level: hashes.len(),
            content: inlines,
        })
}

pub fn reference_definition_parser() -> impl Parser<char, Markdown, Error = Simple<char>> {
    let id = just("[^")
        .ignore_then(take_until(just(']').ignored()).map(|(content, _)| String::from_iter(content)))
        .then_ignore(just(":"));

    let content = inline_parser()
        .repeated()
        .then(
            just('\n')
                .then(just(' ').repeated().at_least(1))
                .ignore_then(inline_parser().repeated())
                .repeated(),
        )
        .map(|(first_line, rest_lines)| {
            let mut all_lines = first_line;
            for line in rest_lines {
                all_lines.extend(line);
            }
            all_lines
        });

    id.then_ignore(text::newline().or_not())
        .then(content)
        .then_ignore(text::newline())
        .map(|(id, content)| Markdown::ReferenceDefinition { id, content })
        .labelled("Reference Definition")
}

pub fn code_block_parser() -> impl Parser<char, Markdown, Error = Simple<char>> {
    just("```")
        .then(take_until(just("```")))
        .map_err_with_span(|_err, span| Simple::custom(span, "Malformed Code Block"))
        .map(|(_, (code, _))| Markdown::CodeBlock(code.iter().collect()))
}

pub fn reference_parser() -> impl Parser<char, InlineMarkdown, Error = Simple<char>> {
    just("[^")
        .ignore_then(take_until(just(']')).map(|(content, _)| String::from_iter(content)))
        .map_err_with_span(|_err, span| Simple::custom(span, "Malformed Reference"))
        .map(InlineMarkdown::Reference)
        .labelled("Reference")
}

pub fn wikilink_parser() -> impl Parser<char, InlineMarkdown, Error = Simple<char>> {
    just("[[")
        .ignore_then(
            take_until(just("]]").ignored()).map(|(content, _)| String::from_iter(content)),
        )
        .map_err_with_span(|_err, span| Simple::custom(span, "Malformed WikiLink"))
        .map(|content| {
            if let Some(pipe_pos) = content.find('|') {
                let target = content[..pipe_pos].trim().to_string();
                let alias = Some(content[pipe_pos + 1..].trim().to_string());
                InlineMarkdown::WikiLink { target, alias }
            } else {
                InlineMarkdown::WikiLink {
                    target: content.trim().to_string(),
                    alias: None,
                }
            }
        })
        .labelled("WikiLink")
}

pub fn link_parser() -> impl Parser<char, InlineMarkdown, Error = Simple<char>> {
    just('[')
        .ignore_then(take_until(just(']')).map(|(content, _)| String::from_iter(content)))
        .then(
            just('(')
                .ignore_then(take_until(just(')')).map(|(content, _)| String::from_iter(content)))
                .map_err_with_span(|_err, span| Simple::custom(span, "Malformed link URL")),
        )
        .map_err_with_span(|_err, span| Simple::custom(span, "Malformed Link"))
        .map(|(text, url)| InlineMarkdown::Link { text, url })
        .labelled("Link")
}

pub fn image_parser() -> impl Parser<char, InlineMarkdown, Error = Simple<char>> {
    just('!')
        .ignore_then(just('[').ignore_then(
            take_until(just(']').ignored()).map(|(content, _)| String::from_iter(content)),
        ))
        .then(just('(').ignore_then(
            take_until(just(')').ignored()).map(|(content, _)| String::from_iter(content)),
        ))
        .map_err_with_span(|_err, span| Simple::custom(span, "Malformed Image"))
        .map(|(alt_text, url)| InlineMarkdown::Image { alt_text, url })
        .labelled("Image")
}

pub fn plain_text_parser() -> impl Parser<char, InlineMarkdown, Error = Simple<char>> {
    filter(|c: &char| !['[', '!', '#', '^', '\n'].contains(c))
        .map_err_with_span(|_err, span| Simple::custom(span, "Failed to parse plain_text"))
        .repeated()
        .at_least(1)
        .map(|chars: Vec<char>| InlineMarkdown::PlainText(chars.iter().collect::<String>()))
        .labelled("Plain Text")
}

pub fn inline_parser() -> impl Parser<char, InlineMarkdown, Error = Simple<char>> {
    choice((
        image_parser(),
        link_parser(),
        wikilink_parser(),
        reference_parser(),
        plain_text_parser(),
    ))
    .labelled("Inline parser")
}

pub fn paragraph_parser() -> impl Parser<char, Markdown, Error = Simple<char>> {
    let inline_content = inline_parser().repeated();

    inline_content
        .map(Markdown::Paragraph)
        .then_ignore(text::newline())
}

pub fn markdown_parser() -> impl Parser<char, Vec<SpannedMarkdown>, Error = Simple<char>> {
    let block_level = choice((
        header_parser(),
        code_block_parser(),
        reference_definition_parser(),
        paragraph_parser(),
    ))
    .map_with_span(SpannedMarkdown);

    block_level.repeated().then_ignore(end())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_header_parser() {
        // assert_eq!(header_parser(), [])
    }
}
