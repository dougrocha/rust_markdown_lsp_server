use chumsky::Parser;
use lib_parser::{
    InlineMarkdownNode, LinkType, MarkdownNode,
    markdown::{header_parser, list_item_parser},
};

use crate::common::{compare, parse_unwrap};

mod common;

#[test]
fn test_headers() {
    compare(
        header_parser(),
        "# Header One",
        MarkdownNode::Header {
            level: 1,
            content: "Header One",
        },
    );

    compare(
        header_parser(),
        "###### Deep",
        MarkdownNode::Header {
            level: 6,
            content: "Deep",
        },
    );
}

#[test]
fn test_lists() {
    let input = "- [x] Done";
    let parsed = parse_unwrap(input);
    if let MarkdownNode::ListItem { checkbox, .. } = &parsed.body[0].0 {
        assert_eq!(*checkbox, Some(true));
    } else {
        panic!("Expected list item");
    }

    let (output, errors) = list_item_parser().parse("- Item").into_output_errors();
    assert!(errors.is_empty(), "Parser failed: {:?}", errors);

    if let Some(MarkdownNode::ListItem { checkbox, content }) = output {
        assert_eq!(checkbox, None);
        // Only check the value (.0), ignoring the span (.1)
        assert_eq!(content[0].0, InlineMarkdownNode::PlainText("Item"));
    } else {
        panic!("Expected Unordered ListItem");
    }
}

#[test]
fn test_inline_elements() {
    let input = "Paragraph with #tag, ![Img](img.png), and [[Wiki|Alias]].";
    let doc = parse_unwrap(input);

    let body = &doc.body[0].0;
    if let MarkdownNode::Paragraph(nodes) = body {
        // We expect: Text, Tag, Text, Image, Text, WikiLink, Text

        let has_tag = nodes
            .iter()
            .any(|n| matches!(n.0, InlineMarkdownNode::Tag("tag")));
        assert!(has_tag, "Failed to find #tag");

        // Check Image
        let has_img = nodes.iter().any(|n| {
            matches!(
                n.0,
                InlineMarkdownNode::Link(LinkType::ImageLink {
                    text: "Img",
                    uri: "img.png",
                    ..
                })
            )
        });
        assert!(has_img, "Failed to find image");

        // Check WikiLink
        let has_wiki = nodes.iter().any(|n| {
            matches!(
                n.0,
                InlineMarkdownNode::Link(LinkType::WikiLink {
                    target: "Wiki",
                    display_text: Some("Alias"),
                    ..
                })
            )
        });
        assert!(has_wiki, "Failed to find wikilink");
    } else {
        panic!("Expected paragraph");
    }
}
