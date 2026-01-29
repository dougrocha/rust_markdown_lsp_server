use chumsky::Parser;
use lib_parser::{MarkdownNode, markdown_parser};

fn main() {
    println!("=== Testing error recovery ===\n");

    // Broken wikilink followed by valid content
    let input1 = r#"# Good Header

[[broken wikilink without closing

# Another Header
Valid paragraph with [[good_link]] here.
"#;
    let result1 = markdown_parser().parse(input1).into_output_errors();
    println!("BROKEN WIKILINK:");
    println!("  Parsed: {}", result1.0.is_some());
    println!("  Errors: {}", result1.1.len());
    if let Some(parsed) = result1.0 {
        println!("  Body nodes recovered: {}", parsed.body.len());
        for (i, node) in parsed.body.iter().enumerate() {
            match &node.0 {
                MarkdownNode::Header { level, content } => {
                    println!("    [{}] Header({}): {}", i, level, content);
                }
                MarkdownNode::Invalid => {
                    println!("    [{}] Invalid", i);
                }
                _ => {
                    println!("    [{}] {:?}", i, std::mem::discriminant(&node.0));
                }
            }
        }
    }

    // Multiple errors in sequence
    let input2 = r#"# Header 1
[[broken1
[[broken2
# Header 2
Valid content
"#;
    let result2 = markdown_parser().parse(input2).into_output_errors();
    println!("\nMULTIPLE ERRORS:");
    println!("  Parsed: {}", result2.0.is_some());
    println!("  Errors: {}", result2.1.len());
    if let Some(parsed) = result2.0 {
        println!("  Body nodes: {}", parsed.body.len());
    }

    // Broken link at end of file
    let input3 = "# Header\n\nParagraph with [[broken";
    let result3 = markdown_parser().parse(input3).into_output_errors();
    println!("\nBROKEN AT EOF:");
    println!("  Parsed: {}", result3.0.is_some());
    println!("  Errors: {}", result3.1.len());
    if let Some(parsed) = result3.0 {
        println!("  Body nodes: {}", parsed.body.len());
    }

    // Deeply nested errors
    let input4 = r#"# Header
[broken markdown link](url
More text [[another broken
Even more [[valid]] content
"#;
    let result4 = markdown_parser().parse(input4).into_output_errors();
    println!("\nDEEPLY NESTED ERRORS:");
    println!("  Parsed: {}", result4.0.is_some());
    println!("  Errors: {}", result4.1.len());
    if let Some(parsed) = result4.0 {
        println!("  Body nodes: {}", parsed.body.len());
        for (i, node) in parsed.body.iter().enumerate() {
            match &node.0 {
                MarkdownNode::Invalid => {
                    println!("    [{}] Invalid", i);
                }
                _ => {}
            }
        }
    }
}
