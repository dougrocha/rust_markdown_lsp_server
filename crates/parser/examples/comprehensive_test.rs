use chumsky::Parser;
use parser::{markdown_parser, MarkdownNode, InlineMarkdownNode};

fn test_input(name: &str, input: &str) {
    println!("\n=== {} ===", name);
    let result = markdown_parser().parse(input).into_output_errors();
    println!("Parsed: {} | Errors: {}", result.0.is_some(), result.1.len());
    
    if let Some(parsed) = result.0 {
        println!("Body nodes: {}", parsed.body.len());
        for (i, node) in parsed.body.iter().take(3).enumerate() {
            match &node.0 {
                MarkdownNode::Header { level, content } => {
                    println!("  [{}] Header({}) '{}'", i, level, content);
                }
                MarkdownNode::Paragraph(inlines) => {
                    println!("  [{}] Paragraph ({} inlines)", i, inlines.len());
                    for (j, inline) in inlines.iter().take(3).enumerate() {
                        match &inline.0 {
                            InlineMarkdownNode::PlainText(t) => {
                                let preview = if t.len() > 30 { format!("{}...", &t[..30]) } else { t.to_string() };
                                println!("      [{}] Text: '{}'", j, preview.replace("\n", "\\n"));
                            }
                            InlineMarkdownNode::Link(_) => println!("      [{}] Link", j),
                            InlineMarkdownNode::Tag(t) => println!("      [{}] Tag: {}", j, t),
                            InlineMarkdownNode::Footnote(f) => println!("      [{}] Footnote: {}", j, f),
                            _ => {}
                        }
                    }
                }
                MarkdownNode::FootnoteDefinition { id, .. } => {
                    println!("  [{}] FootnoteDef: {}", i, id);
                }
                MarkdownNode::Invalid => println!("  [{}] Invalid", i),
            }
        }
    }
    
    for (i, err) in result.1.iter().take(2).enumerate() {
        println!("  Error[{}]: {}", i, err.reason());
    }
}

fn main() {
    // Block elements
    test_input("Ordered List", "1. First\n2. Second\n3. Third");
    test_input("Unordered List", "- Item A\n* Item B\n+ Item C");
    test_input("Task List", "- [ ] Todo\n- [x] Done");
    test_input("Code Block", "```rust\nfn main() {}\n```");
    test_input("Blockquote", "> Quote line 1\n> Quote line 2");
    test_input("Table", "| A | B |\n|---|---|\n| 1 | 2 |");
    test_input("HR", "---\n\nContent\n\n***");
    test_input("HTML Block", "<div>\n  <p>HTML</p>\n</div>");
    
    // Inline elements
    test_input("Bold", "Text with **bold** and __bold__");
    test_input("Italic", "Text with *italic* and _italic_");
    test_input("Bold+Italic", "Text with ***both*** and ___both___");
    test_input("Strikethrough", "Text with ~~strike~~");
    test_input("Inline Code", "Text with `code` here");
    test_input("Autolink", "Visit https://example.com now");
    test_input("HTML Inline", "Text with <span>HTML</span>");
    
    // Links and images
    test_input("Image", "![alt](image.png)");
    test_input("Image with title", "![alt](img.png \"title\")");
    test_input("Reference link", "[text][ref]\n\n[ref]: url");
    test_input("Autolink brackets", "<https://example.com>");
    
    // Special cases
    test_input("Escaped chars", "\\*not bold\\* \\[not link\\]");
    test_input("Line break", "Line 1  \nLine 2");
    test_input("Hard break", "Line 1<br>Line 2");
    
    // Complex nesting
    test_input("Nested lists", "- Item\n  - Nested\n    - Deep");
    test_input("List with code", "- Item with `code`\n- Another");
    test_input("Quote with link", "> Quote with [[link]]");
    
    // Edge cases
    test_input("Multiple blank lines", "Para 1\n\n\n\nPara 2");
    test_input("Windows CRLF", "Line 1\r\nLine 2\r\n");
    test_input("Mixed indentation", "# Header\n\t\tText with tabs");
    test_input("Unicode", "# 日本語ヘッダー\n\n[[リンク]] with 🎉");
    test_input("Very long line", &"x".repeat(1000));
    test_input("Empty input", "");
    test_input("Only whitespace", "   \n\t\n   ");
}
