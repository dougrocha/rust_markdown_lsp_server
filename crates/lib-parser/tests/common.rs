use chumsky::prelude::*;
use lib_parser::ParseError;

pub fn compare<'a, T: std::fmt::Debug + PartialEq>(
    parser: impl Parser<'a, &'a str, T, ParseError<'a>>,
    input: &'a str,
    expected: T,
) {
    let (output, errors) = parser.parse(input).into_output_errors();
    if !errors.is_empty() {
        panic!("Parser failed: {errors:?}");
    }

    let output = output.expect("Parser returned no output despite no errors");
    assert_eq!(output, expected);
}
