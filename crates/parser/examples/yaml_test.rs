use chumsky::Parser;
use parser::yaml::{yaml_parser, Frontmatter, Yaml};

fn main() {
    println!("=== Testing YAML frontmatter parser ===\n");
    
    // Test nested objects (unsupported?)
    let input1 = r#"---
author:
  name: John Doe
  email: john@example.com
---"#;
    let result1 = yaml_parser().parse(input1).into_output_errors();
    println!("NESTED OBJECT:");
    println!("  Parsed: {}", result1.0.is_some());
    println!("  Errors: {}\n", result1.1.len());
    
    // Test array of objects (unsupported?)
    let input2 = r#"---
tags:
  - name: rust
  - name: programming
---"#;
    let result2 = yaml_parser().parse(input2).into_output_errors();
    println!("ARRAY OF OBJECTS:");
    println!("  Parsed: {}", result2.0.is_some());
    println!("  Errors: {}\n", result2.1.len());
    
    // Test numbers
    let input3 = r#"---
count: 123
price: 45.67
---"#;
    let result3 = yaml_parser().parse(input3).into_output_errors();
    println!("NUMBERS:");
    println!("  Parsed: {}", result3.0.is_some());
    if let Some(fm) = result3.0 {
        for (key, value) in &fm.0 {
            match value {
                Yaml::String(s) => println!("  {}: String('{}')", key, s),
                Yaml::List(items) => println!("  {}: List({:?})", key, items),
            }
        }
    }
    println!("  Errors: {}\n", result3.1.len());
    
    // Test booleans
    let input4 = r#"---
draft: true
published: false
---"#;
    let result4 = yaml_parser().parse(input4).into_output_errors();
    println!("BOOLEANS:");
    println!("  Parsed: {}", result4.0.is_some());
    if let Some(fm) = result4.0 {
        for (key, value) in &fm.0 {
            match value {
                Yaml::String(s) => println!("  {}: String('{}')", key, s),
                Yaml::List(items) => println!("  {}: List({:?})", key, items),
            }
        }
    }
    println!("  Errors: {}\n", result4.1.len());
    
    // Test null
    let input5 = r#"---
value: null
empty:
---"#;
    let result5 = yaml_parser().parse(input5).into_output_errors();
    println!("NULL/EMPTY:");
    println!("  Parsed: {}", result5.0.is_some());
    println!("  Errors: {}\n", result5.1.len());
    
    // Test multiline strings
    let input6 = r#"---
description: |
  This is a multiline
  string value
---"#;
    let result6 = yaml_parser().parse(input6).into_output_errors();
    println!("MULTILINE STRING:");
    println!("  Parsed: {}", result6.0.is_some());
    println!("  Errors: {}\n", result6.1.len());
    
    // Test comments
    let input7 = r#"---
# This is a comment
title: My Title # inline comment
tags: example
---"#;
    let result7 = yaml_parser().parse(input7).into_output_errors();
    println!("COMMENTS:");
    println!("  Parsed: {}", result7.0.is_some());
    println!("  Errors: {}", result7.1.len());
}
