use chumsky::Parser;
use lib_parser::{InlineMarkdownNode, LinkType, MarkdownNode, markdown_parser};

fn main() {
    let input = r#"---
title: Test
---

# Header 1

This is a paragraph with [[wikilink]] and [inline](link.md) links.

## Header 2

Another paragraph with #tag and [^1] footnote.

[^1]: Footnote definition here
"#;

    let result = markdown_parser().parse(input).into_output_errors();

    println!("=== Span Accuracy Test ===\n");
    println!("Input length: {} bytes\n", input.len());

    if let Some(parsed) = result.0 {
        if let Some(fm) = &parsed.frontmatter {
            println!("Frontmatter: {} fields", fm.0.len());
        }

        println!("\nBody nodes:");
        for (i, spanned) in parsed.body.iter().enumerate() {
            let span = spanned.1;
            let byte_range = span.start..span.end;
            let actual_text = &input[byte_range.clone()];

            match &spanned.0 {
                MarkdownNode::Header { level, content } => {
                    println!(
                        "\n[{}] Header {} at bytes {}..{}",
                        i, level, span.start, span.end
                    );
                    println!("    Content: '{}'", content);
                    println!("    Actual span text: '{}'", actual_text);
                    println!("    Match: {}", actual_text.contains(content));
                }
                MarkdownNode::Paragraph(inlines) => {
                    println!("\n[{}] Paragraph at bytes {}..{}", i, span.start, span.end);
                    println!("    Actual span text: '{}'", actual_text);
                    println!("    Inline elements: {}", inlines.len());

                    for (j, inline) in inlines.iter().enumerate() {
                        let inline_span = inline.1;
                        let inline_range = inline_span.start..inline_span.end;
                        let inline_text = &input[inline_range];

                        match &inline.0 {
                            InlineMarkdownNode::PlainText(text) => {
                                println!(
                                    "      [{}] PlainText '{}' at {}..{}",
                                    j, text, inline_span.start, inline_span.end
                                );
                                println!("          Actual: '{}'", inline_text);
                                println!("          Match: {}", text == &inline_text);
                            }
                            InlineMarkdownNode::Link(link) => match link {
                                LinkType::WikiLink {
                                    target,
                                    display_text: _,
                                    header: _,
                                } => {
                                    println!(
                                        "      [{}] WikiLink '{}' at {}..{}",
                                        j, target, inline_span.start, inline_span.end
                                    );
                                    println!("          Actual: '{}'", inline_text);
                                    println!("          Match: {}", inline_text.contains(target));
                                }
                                LinkType::InlineLink {
                                    text: _,
                                    uri,
                                    header: _,
                                } => {
                                    println!(
                                        "      [{}] InlineLink '{}' at {}..{}",
                                        j, uri, inline_span.start, inline_span.end
                                    );
                                    println!("          Actual: '{}'", inline_text);
                                    println!("          Match: {}", inline_text.contains(uri));
                                }
                            },
                            InlineMarkdownNode::Tag(tag) => {
                                println!(
                                    "      [{}] Tag '{}' at {}..{}",
                                    j, tag, inline_span.start, inline_span.end
                                );
                                println!("          Actual: '{}'", inline_text);
                            }
                            InlineMarkdownNode::Footnote(id) => {
                                println!(
                                    "      [{}] Footnote '{}' at {}..{}",
                                    j, id, inline_span.start, inline_span.end
                                );
                                println!("          Actual: '{}'", inline_text);
                            }
                            _ => {}
                        }
                    }
                }
                MarkdownNode::FootnoteDefinition { id, content: _ } => {
                    println!(
                        "\n[{}] FootnoteDef '{}' at bytes {}..{}",
                        i, id, span.start, span.end
                    );
                    println!("    Actual span text: '{}'", actual_text);
                }
                _ => {}
            }
        }
    }

    println!("\n\nErrors: {}", result.1.len());
}
