use common::compare;
use lib_parser::{InlineMarkdownNode, LinkType, MarkdownNode, markdown::*};

mod common;

#[test]
fn test_header_parser() {
    let input = "# Header Text";
    let expected = MarkdownNode::Header {
        level: 1,
        content: "Header Text",
    };

    compare(header_parser(), input, expected);
}

#[test]
fn test_wikilink_parser_basic() {
    let input = "[[target]]";
    let expected = InlineMarkdownNode::Link(LinkType::WikiLink {
        target: "target",
        display_text: None,
        header: None,
    });
    compare(wikilink_parser(), input, expected);
}

#[test]
fn test_link_parser_basic() {
    let input = "[Link Text](http://example.com)";
    let expected = InlineMarkdownNode::Link(LinkType::InlineLink {
        text: "Link Text",
        uri: "http://example.com",
        header: None,
    });
    compare(link_parser(), input, expected);
}

#[test]
fn test_footnote_parser_basic() {
    let input = "[^1]";
    let expected = InlineMarkdownNode::Footnote("1");
    compare(footnote_parser(), input, expected);
}

#[test]
fn test_tag_parser() {
    let input = "#tag123";
    let expected = InlineMarkdownNode::Tag("tag123");
    compare(tag_parser(), input, expected);
}
