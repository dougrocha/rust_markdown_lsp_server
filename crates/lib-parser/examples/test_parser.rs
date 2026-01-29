use chumsky::Parser;
use lib_parser::markdown_parser;

fn main() {
    // Test basic parsing
    let input = "# Header\n\nSome text [[wikilink]] and [inline](link.md)\n";
    let result = markdown_parser().parse(input).into_output_errors();
    println!("=== Basic parsing ===");
    println!("Parsed: {:?}\n", result.0.is_some());

    // Test malformed wikilink (no closing brackets)
    let input2 = "# Header\n\n[[broken link\n\nMore text";
    let result2 = markdown_parser().parse(input2).into_output_errors();
    println!("=== Malformed wikilink ===");
    println!(
        "Parsed: {:?}, errors: {}",
        result2.0.is_some(),
        result2.1.len()
    );

    // Test malformed inline link (no closing paren)
    let input3 = "# Header\n\n[text](broken\n\nMore text";
    let result3 = markdown_parser().parse(input3).into_output_errors();
    println!("\n=== Malformed inline link ===");
    println!(
        "Parsed: {:?}, errors: {}",
        result3.0.is_some(),
        result3.1.len()
    );

    // Test code block (unsupported)
    let input4 = "# Header\n\n```rust\nfn main() {}\n```\n\nParagraph";
    let result4 = markdown_parser().parse(input4).into_output_errors();
    println!("\n=== Code block (unsupported) ===");
    println!(
        "Parsed: {:?}, errors: {}",
        result4.0.is_some(),
        result4.1.len()
    );
    if let Some(parsed) = result4.0 {
        println!("  Body nodes: {}", parsed.body.len());
        for (i, node) in parsed.body.iter().enumerate() {
            println!("    Node {}: {:?}", i, std::mem::discriminant(&node.0));
        }
    }

    // Test list (unsupported)
    let input5 = "# Header\n\n- Item 1\n- Item 2\n";
    let result5 = markdown_parser().parse(input5).into_output_errors();
    println!("\n=== List (unsupported) ===");
    println!(
        "Parsed: {:?}, errors: {}",
        result5.0.is_some(),
        result5.1.len()
    );
    if let Some(parsed) = result5.0 {
        println!("  Body nodes: {}", parsed.body.len());
        for (i, node) in parsed.body.iter().enumerate() {
            println!("    Node {}: {:?}", i, std::mem::discriminant(&node.0));
        }
    }

    // Test blockquote (unsupported)
    let input6 = "# Header\n\n> Quote text\n> More quote\n";
    let result6 = markdown_parser().parse(input6).into_output_errors();
    println!("\n=== Blockquote (unsupported) ===");
    println!(
        "Parsed: {:?}, errors: {}",
        result6.0.is_some(),
        result6.1.len()
    );
}
