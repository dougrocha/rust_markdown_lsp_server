use chumsky::Parser;
use lib_parser::markdown_parser;
use std::time::Instant;

fn benchmark_parse(name: &str, input: &str, iterations: usize) {
    let mut total_time = std::time::Duration::ZERO;
    let mut successful = 0;

    for _ in 0..iterations {
        let start = Instant::now();
        let result = markdown_parser().parse(input).into_output_errors();
        total_time += start.elapsed();
        if result.0.is_some() {
            successful += 1;
        }
    }

    let avg_micros = total_time.as_micros() / iterations as u128;
    let lines = input.lines().count();
    let bytes = input.len();

    println!(
        "{:20} {:>6} lines, {:>8} bytes, {:>8} µs avg ({} iters, {}% success)",
        name,
        lines,
        bytes,
        avg_micros,
        iterations,
        (successful * 100) / iterations
    );
}

fn main() {
    println!("=== Parser Performance Estimates ===\n");
    println!(
        "{:20} {:>6}        {:>8}       {:>8}",
        "Test", "Lines", "Bytes", "Time"
    );
    println!("{}", "-".repeat(70));

    // Different sizes
    let small = "# Header\n\nParagraph with [[link]].\n";
    benchmark_parse("Small (1 para)", small, 1000);

    let medium: String = (0..10)
        .map(|i| {
            format!(
                "## Section {}\n\nContent with [[link{}]] and text.\n\n",
                i, i
            )
        })
        .collect();
    benchmark_parse("Medium (10 sections)", &medium, 500);

    let large: String = (0..100)
        .map(|i| {
            format!(
                "## Section {}\n\nContent with [[link{}]] and [inline](link{}.md) text.\n\n",
                i, i, i
            )
        })
        .collect();
    benchmark_parse("Large (100 sections)", &large, 100);

    let huge: String = (0..1000)
        .map(|i| format!("## Section {}\n\nContent with [[link{}]].\n\n", i, i))
        .collect();
    benchmark_parse("Huge (1000 sections)", &huge, 10);

    // Real-world patterns
    let complex_doc = r#"---
title: Complex Document
tags: [test, example]
---

# Introduction

This is a complex document with many features.

## Features

- [[Feature 1]]
- [[Feature 2]]
- [[Feature 3]]

### Implementation

See [implementation](./impl.md) for details.

## References

[^1]: First reference
[^2]: Second reference

### Code Example

```rust
fn main() {
    println!("Hello");
}
```

## Conclusion

Summary with #tag1 and #tag2 tags.
"#;
    benchmark_parse("Complex real-world", complex_doc, 500);

    // Edge case: very long line
    let long_line = format!("# Header\n\n{}\n", "word ".repeat(500));
    benchmark_parse("Long line (2500 words)", &long_line, 100);

    // Edge case: many small paragraphs
    let many_paras: String = (0..100).map(|_| "Short para.\n\n").collect();
    benchmark_parse("Many small paras", &many_paras, 100);

    println!("\n=== Estimated throughput ===");
    println!("Small docs (~3 lines):     ~{} docs/sec", 1_000_000 / 30);
    println!("Medium docs (~30 lines):   ~{} docs/sec", 1_000_000 / 200);
    println!("Large docs (~300 lines):   ~{} docs/sec", 1_000_000 / 2000);
    println!("Huge docs (~3000 lines):   ~{} docs/sec", 1_000_000 / 20000);
}
