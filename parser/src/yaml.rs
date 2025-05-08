use chumsky::prelude::*;

#[derive(Debug, Clone)]
pub enum Yaml<'a> {
    String(&'a str),
    Array(Vec<&'a str>),
}

#[derive(Debug, Clone)]
pub struct Frontmatter<'a>(Vec<(&'a str, Yaml<'a>)>);

pub fn frontmatter_parser<'a>(
) -> impl Parser<'a, &'a str, Frontmatter<'a>, extra::Err<Rich<'a, char>>> {
    let key = text::ident().labelled("Key");

    let single_value = any()
        .filter(|c| *c != '\n')
        .repeated()
        .to_slice()
        .map(|v: &'a str| Yaml::String(v.trim()))
        .then_ignore(text::newline())
        .labelled("Value");

    let item = just('-').padded().ignore_then(
        any()
            .filter(|c| *c != '\n')
            .repeated()
            .to_slice()
            .map(|v: &'a str| v.trim())
            .then_ignore(text::newline()),
    );

    let list_item = text::whitespace()
        .ignore_then(item.repeated().at_least(1).collect::<Vec<_>>())
        .map(Yaml::Array);

    let value = list_item.or(single_value);

    let line = key.then_ignore(just(':').padded()).then(value);

    just("---")
        .ignore_then(text::newline())
        .ignore_then(line.repeated().collect::<Vec<_>>())
        .then_ignore(just("---"))
        .then_ignore(text::newline().or_not())
        .map(|entries| {
            println!("{:?}", entries);
            Frontmatter(entries)
        })
}
