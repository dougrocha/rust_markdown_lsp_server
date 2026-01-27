use chumsky::Parser;
use parser::{markdown_parser, yaml::Yaml};

fn main() {
    println!("=== Frontmatter Integration Tests ===\n");

    // Valid frontmatter
    let input1 = r#"---
title: My Note
tags: example
---

# Content
"#;
    let result1 = markdown_parser().parse(input1).into_output_errors();
    println!("Valid frontmatter:");
    if let Some(parsed) = result1.0 {
        if let Some(fm) = &parsed.frontmatter {
            println!("  Fields: {}", fm.0.len());
            for (k, v) in &fm.0 {
                match v {
                    Yaml::String(s) => println!("    {}: {}", k, s),
                    Yaml::List(items) => println!("    {}: {:?}", k, items),
                }
            }
        }
        println!("  Body nodes: {}", parsed.body.len());
    }
    println!("  Errors: {}\n", result1.1.len());

    // Frontmatter with list
    let input2 = r#"---
id: note-123
tags:
  - rust
  - programming
---

# Header
"#;
    let result2 = markdown_parser().parse(input2).into_output_errors();
    println!("Frontmatter with list:");
    if let Some(parsed) = result2.0
        && let Some(fm) = &parsed.frontmatter
    {
        for (k, v) in &fm.0 {
            match v {
                Yaml::String(s) => println!("  {}: String({})", k, s),
                Yaml::List(items) => println!("  {}: List({:?})", k, items),
            }
        }
    }
    println!("  Errors: {}\n", result2.1.len());

    // Invalid frontmatter (should skip)
    let input3 = r#"---
invalid yaml: [unclosed
---

# Content
"#;
    let result3 = markdown_parser().parse(input3).into_output_errors();
    println!("Invalid frontmatter:");
    println!("  Parsed: {}", result3.0.is_some());
    if let Some(parsed) = result3.0 {
        println!("  Has frontmatter: {}", parsed.frontmatter.is_some());
        println!("  Body nodes: {}", parsed.body.len());
    }
    println!("  Errors: {}\n", result3.1.len());

    // No frontmatter
    let input4 = "# Header\n\nContent";
    let result4 = markdown_parser().parse(input4).into_output_errors();
    println!("No frontmatter:");
    if let Some(parsed) = result4.0 {
        println!("  Has frontmatter: {}", parsed.frontmatter.is_some());
        println!("  Body nodes: {}", parsed.body.len());
    }
    println!("  Errors: {}\n", result4.1.len());

    // Frontmatter-like but not at start
    let input5 = r#"# Header

---
title: Not frontmatter
---

Content
"#;
    let result5 = markdown_parser().parse(input5).into_output_errors();
    println!("Frontmatter-like mid-document:");
    if let Some(parsed) = result5.0 {
        println!("  Has frontmatter: {}", parsed.frontmatter.is_some());
        println!("  Body nodes: {}", parsed.body.len());
    }
    println!("  Errors: {}", result5.1.len());
}
