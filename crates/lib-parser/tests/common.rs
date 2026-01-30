use ariadne::{Color, Label, Report, ReportKind, Source};
use chumsky::prelude::*;
use lib_parser::{ParseError, ParsedMarkdown, markdown_parser};

pub fn print_parse_errors(src: &str, errs: Vec<Rich<char>>) {
    errs.into_iter().for_each(|e| {
        Report::build(ReportKind::Error, ("Test Input", e.span().into_range()))
            .with_message(e.to_string())
            .with_label(
                Label::new(("Test Input", e.span().into_range()))
                    .with_message(e.reason().to_string())
                    .with_color(Color::Red),
            )
            .finish()
            .eprint(("Test Input", Source::from(&src)))
            .unwrap()
    });
}

pub fn compare<'a, T: std::fmt::Debug + PartialEq>(
    parser: impl Parser<'a, &'a str, T, ParseError<'a>>,
    input: &'a str,
    expected: T,
) {
    let (output, errors) = parser.parse(input).into_output_errors();
    if !errors.is_empty() {
        print_parse_errors(input, errors);
        panic!("Parser failed (see output above)");
    }

    // Unwrap safely because we checked errors
    let output = output.expect("Parser returned no output despite no errors");
    assert_eq!(output, expected);
}

pub fn parse_unwrap(input: &str) -> ParsedMarkdown<'_> {
    let (output, errors) = markdown_parser().parse(input).into_output_errors();
    if !errors.is_empty() {
        println!("\n=== PARSER FAILED ===");
        print_parse_errors(input, errors);
        println!("=====================\n");
        panic!("Parser failed with errors");
    }
    output.expect("Parser returned no output")
}
