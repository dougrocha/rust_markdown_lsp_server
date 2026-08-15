use chumsky::prelude::*;

pub mod new_markdown;
pub mod yaml;

pub type ParseError<'a> = extra::Err<Rich<'a, char>>;
