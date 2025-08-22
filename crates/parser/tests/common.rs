use ariadne::{Color, Label, Report, ReportKind, Source};
use chumsky::prelude::*;

pub(crate) fn print_parse_errors(src: &str, errs: Vec<Rich<char>>) {
    errs.into_iter().for_each(|e| {
        Report::build(ReportKind::Error, ("Testing File", e.span().into_range()))
            .with_message(e.to_string())
            .with_label(
                Label::new(("Testing File", e.span().into_range()))
                    .with_message(e.reason().to_string())
                    .with_color(Color::Red),
            )
            .finish()
            .eprint(("Testing File", Source::from(&src)))
            .unwrap()
    });
}

pub(crate) fn compare<'a, T: std::fmt::Debug + PartialEq>(
    parser: impl Parser<'a, &'a str, T, extra::Err<Rich<'a, char>>>,
    input: &'a str,
    expected: T,
) {
    let result = parser.parse(input).into_result();
    match result {
        Ok(res) => assert_eq!(res, expected),
        Err(errs) => {
            print_parse_errors(input, errs);
            panic!("Parser failed");
        }
    }
}
