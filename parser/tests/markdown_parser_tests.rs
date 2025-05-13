use chumsky::{error::Rich, span::SimpleSpan};
use common::compare;
use parser::{markdown::*, InlineMarkdown, LinkHeader, Markdown, Parser};

mod common;

#[test]
fn test_header_parser() {
    let input = "# Header Text\n";
    let expected = Markdown::Header {
        level: 1,
        content: &input[2..13],
    };

    compare(header_parser(), input, expected);
}

#[test]
fn test_wikilink_parser_basic() {
    let input = "[[target]]";
    let expected = InlineMarkdown::WikiLink {
        target: "target",
        alias: None,
        header: None,
    };
    compare(wikilink_parser(), input, expected);
}

#[test]
fn test_wikilink_parser_newline_and_alias() {
    let input = "[[tar\nget|alias]]";
    assert!(wikilink_parser().parse(input).has_errors());
}

#[test]
fn test_wikilink_parser_with_header() {
    let input = "[[target#header]]";
    let expected = InlineMarkdown::WikiLink {
        target: "target",
        alias: None,
        header: Some(LinkHeader {
            level: 1,
            content: "header",
        }),
    };
    compare(wikilink_parser(), input, expected);
}

#[test]
fn test_wikilink_parser_empty_alias() {
    let input = "[[target|]]";
    let expected = InlineMarkdown::WikiLink {
        target: "target",
        alias: None,
        header: None,
    };
    compare(wikilink_parser(), input, expected);
}

#[test]
fn test_wikilink_parser_whitespace() {
    let input = "[[ target | alias ]]";
    let expected = InlineMarkdown::WikiLink {
        target: "target",
        alias: Some("alias"),
        header: None,
    };
    let (res, errors) = wikilink_parser().parse(input).into_output_errors();
    assert_eq!(res, Some(expected));

    assert_eq!(
        errors.first(),
        Some(&Rich::custom(
            SimpleSpan::from(11..18),
            "WikiLink alias contains spaces before or after."
        ))
    );
}

#[test]
fn test_link_parser_basic() {
    let input = "[Link Text](http://example.com)";
    let expected = InlineMarkdown::Link {
        title: "Link Text",
        uri: "http://example.com",
        header: None,
    };
    compare(link_parser(), input, expected);
}

#[test]
fn test_link_parser_with_header() {
    let input = "[Link Text](./other_file#Heading)";
    let expected = InlineMarkdown::Link {
        title: "Link Text",
        uri: "./other_file",
        header: Some(LinkHeader {
            level: 1,
            content: "Heading",
        }),
    };
    compare(link_parser(), input, expected);
}

#[test]
fn test_image_parser_basic() {
    let input = "![Alt Text](image.png)";
    let expected = InlineMarkdown::Image {
        alt_text: "Alt Text",
        uri: "image.png",
    };
    compare(image_parser(), input, expected);
}

#[test]
fn test_image_parser_with_spaces() {
    let input = "![Alt Text with spaces](image with spaces.png)";
    let expected = InlineMarkdown::Image {
        alt_text: "Alt Text with spaces",
        uri: "image with spaces.png",
    };
    compare(image_parser(), input, expected);
}

#[test]
fn test_image_parser_empty_url() {
    let input = "![Alt Text]()";
    let expected = InlineMarkdown::Image {
        alt_text: "Alt Text",
        uri: "",
    };
    compare(image_parser(), input, expected);
}

#[test]
fn test_image_parser_invalid_input() {
    let input = "![](image.png)";
    assert!(image_parser().parse(input).has_errors());

    let input = "!()";
    assert!(image_parser().parse(input).has_errors());

    let input = "![Alt Text]image.png";
    assert!(image_parser().parse(input).has_errors());

    let input = "!Alt Text(image.png)";
    assert!(image_parser().parse(input).has_errors());

    let input = "![Alt Text] (image.png)";
    assert!(image_parser().parse(input).has_errors());
}

#[test]
fn test_image_parser_unicode() {
    let input = "![😀](🚀.png)";
    let expected = InlineMarkdown::Image {
        alt_text: "😀",
        uri: "🚀.png",
    };
    compare(image_parser(), input, expected);
}

#[test]
fn test_plain_text_parser_skip_image() {
    let input = "println!('Hello World')";
    let expected = InlineMarkdown::PlainText(input);
    compare(plain_text_parser(), input, expected);
}

#[test]
fn test_footnote_parser_basic() {
    let input = "[^1]";
    let expected = InlineMarkdown::Footnote("1");
    compare(footnote_parser(), input, expected);
}

#[test]
fn test_footnote_parser_alphabetic() {
    let input = "[^abc]";
    let expected = InlineMarkdown::Footnote("abc");
    compare(footnote_parser(), input, expected);
}

#[test]
fn test_footnote_parser_alphanumeric() {
    let input = "[^a1b2]";
    let expected = InlineMarkdown::Footnote("a1b2");
    compare(footnote_parser(), input, expected);
}

#[test]
fn test_footnote_parser_numeric() {
    let input = "[^123]";
    let expected = InlineMarkdown::Footnote("123");
    compare(footnote_parser(), input, expected);
}

#[test]
fn test_footnote_parser_mixed_case() {
    let input = "[^aBc12]";
    let expected = InlineMarkdown::Footnote("aBc12");
    compare(footnote_parser(), input, expected);
}

#[test]
fn test_footnote_parser_empty_label() {
    let input = "[^]";
    assert!(footnote_parser().parse(input).has_errors());
}

#[test]
fn test_footnote_parser_with_space() {
    let input = "[^1 ]";
    assert!(footnote_parser().parse(input).has_errors());
}

#[test]
fn test_footnote_parser_with_hyphen() {
    let input = "[^1-2]";
    assert!(footnote_parser().parse(input).has_errors());
}

#[test]
fn test_footnote_parser_with_period() {
    let input = "[^1.]";
    assert!(footnote_parser().parse(input).has_errors());
}

#[test]
fn test_footnote_parser_double_caret() {
    let input = "[^^]";
    assert!(footnote_parser().parse(input).has_errors());
}

#[test]
fn test_footnote_parser_no_closing_bracket() {
    let input = "[^1";
    assert!(footnote_parser().parse(input).has_errors());
}

#[test]
fn test_footnote_parser_no_opening_bracket() {
    let input = "^1]";
    assert!(footnote_parser().parse(input).has_errors());
}

#[test]
fn test_footnote_parser_no_caret() {
    let input = "[1]";
    assert!(footnote_parser().parse(input).has_errors());
}
