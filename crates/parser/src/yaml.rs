use chumsky::prelude::*;

use crate::ParseError;

#[derive(Debug, Clone, PartialEq)]
pub enum Yaml<'a> {
    String(&'a str),
    List(Vec<&'a str>),
}

type KeyValue<'a> = (&'a str, Yaml<'a>);

#[derive(Debug, Clone, PartialEq)]
pub struct Frontmatter<'a>(pub Vec<KeyValue<'a>>);

fn unquoted_string<'a>() -> impl Parser<'a, &'a str, &'a str, ParseError<'a>> {
    any()
        .filter(|c: &char| !c.is_control() && *c != '\n' && *c != ':' && *c != '#')
        .repeated()
        .at_least(1)
        .to_slice()
}

fn quoted_string<'a>() -> impl Parser<'a, &'a str, &'a str, ParseError<'a>> {
    just('"')
        .ignore_then(any().filter(|c| *c != '"').repeated().to_slice())
        .then_ignore(just('"'))
}

fn string_value<'a>() -> impl Parser<'a, &'a str, &'a str, ParseError<'a>> {
    quoted_string().or(unquoted_string())
}

fn list_item_parser<'a>() -> impl Parser<'a, &'a str, &'a str, ParseError<'a>> {
    text::whitespace()
        .at_least(1)
        .then_ignore(just('-'))
        .then_ignore(text::whitespace())
        .ignore_then(string_value())
}

fn indented_list_parser<'a>() -> impl Parser<'a, &'a str, Yaml<'a>, ParseError<'a>> {
    list_item_parser()
        .separated_by(text::newline())
        .at_least(1)
        .collect::<Vec<_>>()
        .map(Yaml::List)
}

fn string_value_parser<'a>() -> impl Parser<'a, &'a str, Yaml<'a>, ParseError<'a>> {
    string_value().map(Yaml::String)
}

fn key_value_pair_parser<'a>() -> impl Parser<'a, &'a str, KeyValue<'a>, ParseError<'a>> {
    key_parser().then_ignore(just(':')).then(
        text::newline()
            .ignore_then(indented_list_parser())
            .or(text::whitespace().ignore_then(string_value_parser())),
    )
}

fn key_parser<'a>() -> impl Parser<'a, &'a str, &'a str, ParseError<'a>> {
    text::ident().to_slice()
}

fn front_matter_body_parser<'a>() -> impl Parser<'a, &'a str, Frontmatter<'a>, ParseError<'a>> {
    key_value_pair_parser()
        .separated_by(text::newline())
        .at_least(1)
        .collect::<Vec<_>>()
        .map(Frontmatter)
}

fn delimiter<'a>() -> impl Parser<'a, &'a str, (), ParseError<'a>> {
    text::whitespace()
        .or_not()
        .ignore_then(just("---"))
        .then_ignore(text::whitespace().or_not())
        .then_ignore(text::newline().or_not())
        .ignored()
}

pub fn yaml_parser<'a>() -> impl Parser<'a, &'a str, Frontmatter<'a>, ParseError<'a>> {
    delimiter()
        .ignore_then(front_matter_body_parser())
        .then_ignore(text::newline().or_not())
        .then_ignore(delimiter())
}
