use chumsky::Parser;
use lib_parser::{InlineMarkdownNode, MarkdownNode, markdown_parser};

fn main() {
    println!("=== Testing what unsupported markdown becomes ===\n");

    // Test code block
    let input = "# Header\n\n```rust\nfn main() {}\n```\n\nParagraph after";
    let result = markdown_parser().parse(input).into_output_errors();
    println!("CODE BLOCK:");
    if let Some(parsed) = result.0 {
        for (i, spanned) in parsed.body.iter().enumerate() {
            match &spanned.0 {
                MarkdownNode::Header { level, content } => {
                    println!("  [{}] Header({}): '{}'", i, level, content);
                }
                MarkdownNode::Paragraph(inlines) => {
                    println!(
                        "  [{}] Paragraph with {} inline elements:",
                        i,
                        inlines.len()
                    );
                    for (j, inline) in inlines.iter().enumerate() {
                        match &inline.0 {
                            InlineMarkdownNode::PlainText(text) => {
                                println!(
                                    "      [{}] PlainText: '{}'",
                                    j,
                                    text.replace("\n", "\\n")
                                );
                            }
                            other => println!("      [{}] {:?}", j, other),
                        }
                    }
                }
                MarkdownNode::FootnoteDefinition { id, content: _ } => {
                    println!("  [{}] FootnoteDefinition({})", i, id);
                }
                MarkdownNode::ListItem { checkbox, content } => {
                    let checkbox_str = match checkbox {
                        Some(true) => "[x]",
                        Some(false) => "[ ]",
                        None => "   ",
                    };
                    println!(
                        "  [{}] ListItem {} with {} inline elements",
                        i,
                        checkbox_str,
                        content.len()
                    );
                }
                MarkdownNode::Invalid => {
                    println!("  [{}] Invalid", i);
                }
            }
        }
    }
    println!("  Errors: {}\n", result.1.len());

    // Test list
    let input2 = "# Header\n\n- Item 1\n- Item 2\n- Item 3";
    let result2 = markdown_parser().parse(input2).into_output_errors();
    println!("LIST:");
    if let Some(parsed) = result2.0 {
        for (i, spanned) in parsed.body.iter().enumerate() {
            match &spanned.0 {
                MarkdownNode::Header { level, content } => {
                    println!("  [{}] Header({}): '{}'", i, level, content);
                }
                MarkdownNode::Paragraph(inlines) => {
                    println!(
                        "  [{}] Paragraph with {} inline elements:",
                        i,
                        inlines.len()
                    );
                    for (j, inline) in inlines.iter().enumerate() {
                        match &inline.0 {
                            InlineMarkdownNode::PlainText(text) => {
                                println!(
                                    "      [{}] PlainText: '{}'",
                                    j,
                                    text.replace("\n", "\\n")
                                );
                            }
                            other => println!("      [{}] {:?}", j, other),
                        }
                    }
                }
                _ => println!("  [{}] Other", i),
            }
        }
    }
    println!("  Errors: {}\n", result2.1.len());

    // Test blockquote
    let input3 = "# Header\n\n> This is a quote\n> Multiple lines";
    let result3 = markdown_parser().parse(input3).into_output_errors();
    println!("BLOCKQUOTE:");
    if let Some(parsed) = result3.0 {
        for (i, spanned) in parsed.body.iter().enumerate() {
            match &spanned.0 {
                MarkdownNode::Header { level, content } => {
                    println!("  [{}] Header({}): '{}'", i, level, content);
                }
                MarkdownNode::Paragraph(inlines) => {
                    println!(
                        "  [{}] Paragraph with {} inline elements:",
                        i,
                        inlines.len()
                    );
                    for (j, inline) in inlines.iter().enumerate() {
                        match &inline.0 {
                            InlineMarkdownNode::PlainText(text) => {
                                println!(
                                    "      [{}] PlainText: '{}'",
                                    j,
                                    text.replace("\n", "\\n")
                                );
                            }
                            other => println!("      [{}] {:?}", j, other),
                        }
                    }
                }
                _ => println!("  [{}] Other", i),
            }
        }
    }
    println!("  Errors: {}\n", result3.1.len());

    // Test table
    let input4 = "# Header\n\n| Col1 | Col2 |\n|------|------|\n| A    | B    |";
    let result4 = markdown_parser().parse(input4).into_output_errors();
    println!("TABLE:");
    if let Some(parsed) = result4.0 {
        for (i, spanned) in parsed.body.iter().enumerate() {
            match &spanned.0 {
                MarkdownNode::Header { level, content } => {
                    println!("  [{}] Header({}): '{}'", i, level, content);
                }
                MarkdownNode::Paragraph(inlines) => {
                    println!(
                        "  [{}] Paragraph with {} inline elements:",
                        i,
                        inlines.len()
                    );
                    for (j, inline) in inlines.iter().enumerate() {
                        match &inline.0 {
                            InlineMarkdownNode::PlainText(text) => {
                                let preview = if text.len() > 50 {
                                    format!("{}...", &text[..50])
                                } else {
                                    text.to_string()
                                };
                                println!(
                                    "      [{}] PlainText: '{}'",
                                    j,
                                    preview.replace("\n", "\\n")
                                );
                            }
                            other => println!("      [{}] {:?}", j, other),
                        }
                    }
                }
                _ => println!("  [{}] Other", i),
            }
        }
    }
    println!("  Errors: {}", result4.1.len());
}
