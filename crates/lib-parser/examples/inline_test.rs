use chumsky::Parser;
use lib_parser::{InlineMarkdownNode, MarkdownNode, markdown_parser};

fn main() {
    println!("=== Testing inline markdown elements ===\n");

    // Test inline code
    let input = "Text with `inline code` here";
    let result = markdown_parser().parse(input).into_output_errors();
    println!("INLINE CODE: '{}'", input);
    if let Some(parsed) = result.0 {
        if let Some(spanned) = parsed.body.first() {
            if let MarkdownNode::Paragraph(inlines) = &spanned.0 {
                println!("  Inline elements: {}", inlines.len());
                for (i, inline) in inlines.iter().enumerate() {
                    match &inline.0 {
                        InlineMarkdownNode::PlainText(text) => {
                            println!("    [{}] PlainText: '{}'", i, text);
                        }
                        other => println!("    [{}] {:?}", i, other),
                    }
                }
            }
        }
    }
    println!("  Errors: {}\n", result.1.len());

    // Test bold
    let input2 = "Text with **bold** and __also bold__ here";
    let result2 = markdown_parser().parse(input2).into_output_errors();
    println!("BOLD: '{}'", input2);
    if let Some(parsed) = result2.0 {
        if let Some(spanned) = parsed.body.first() {
            if let MarkdownNode::Paragraph(inlines) = &spanned.0 {
                println!("  Inline elements: {}", inlines.len());
                for (i, inline) in inlines.iter().enumerate() {
                    match &inline.0 {
                        InlineMarkdownNode::PlainText(text) => {
                            println!("    [{}] PlainText: '{}'", i, text);
                        }
                        other => println!("    [{}] {:?}", i, other),
                    }
                }
            }
        }
    }
    println!("  Errors: {}\n", result2.1.len());

    // Test italic
    let input3 = "Text with *italic* and _also italic_ here";
    let result3 = markdown_parser().parse(input3).into_output_errors();
    println!("ITALIC: '{}'", input3);
    if let Some(parsed) = result3.0 {
        if let Some(spanned) = parsed.body.first() {
            if let MarkdownNode::Paragraph(inlines) = &spanned.0 {
                println!("  Inline elements: {}", inlines.len());
                for (i, inline) in inlines.iter().enumerate() {
                    match &inline.0 {
                        InlineMarkdownNode::PlainText(text) => {
                            println!("    [{}] PlainText: '{}'", i, text);
                        }
                        other => println!("    [{}] {:?}", i, other),
                    }
                }
            }
        }
    }
    println!("  Errors: {}\n", result3.1.len());

    // Test strikethrough
    let input4 = "Text with ~~strikethrough~~ here";
    let result4 = markdown_parser().parse(input4).into_output_errors();
    println!("STRIKETHROUGH: '{}'", input4);
    if let Some(parsed) = result4.0 {
        if let Some(spanned) = parsed.body.first() {
            if let MarkdownNode::Paragraph(inlines) = &spanned.0 {
                println!("  Inline elements: {}", inlines.len());
                for (i, inline) in inlines.iter().enumerate() {
                    match &inline.0 {
                        InlineMarkdownNode::PlainText(text) => {
                            println!("    [{}] PlainText: '{}'", i, text);
                        }
                        other => println!("    [{}] {:?}", i, other),
                    }
                }
            }
        }
    }
    println!("  Errors: {}\n", result4.1.len());

    // Test mixed formatting
    let input5 = "Text with **bold _and italic_** and `code`";
    let result5 = markdown_parser().parse(input5).into_output_errors();
    println!("MIXED: '{}'", input5);
    if let Some(parsed) = result5.0 {
        if let Some(spanned) = parsed.body.first() {
            if let MarkdownNode::Paragraph(inlines) = &spanned.0 {
                println!("  Inline elements: {}", inlines.len());
                for (i, inline) in inlines.iter().enumerate() {
                    match &inline.0 {
                        InlineMarkdownNode::PlainText(text) => {
                            println!("    [{}] PlainText: '{}'", i, text);
                        }
                        other => println!("    [{}] {:?}", i, other),
                    }
                }
            }
        }
    }
    println!("  Errors: {}", result5.1.len());
}
