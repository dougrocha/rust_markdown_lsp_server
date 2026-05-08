use chumsky::Parser;
use lib_parser::markdown_parser;
use lib_parser::new_markdown::Parser as NewParser;
use std::fs;

fn main() {
    divan::main();
}

// Old parser benchmarks

#[divan::bench(min_time = 2)]
fn old_parse_small_markdown_file() {
    let content = fs::read_to_string("benches/test_small.md").unwrap();

    divan::black_box(markdown_parser().parse(&content));
}

#[divan::bench(min_time = 2)]
fn old_parse_medium_markdown_file() {
    let content = fs::read_to_string("benches/test_medium.md").unwrap();

    divan::black_box(markdown_parser().parse(&content));
}

#[divan::bench(min_time = 2)]
fn old_parse_large_markdown_file() {
    let content = fs::read_to_string("benches/test_large.md").unwrap();

    divan::black_box(markdown_parser().parse(&content));
}

#[divan::bench(min_time = 2)]
fn old_parse_multiple_files_sequentially() {
    let files = vec![
        "benches/test_small.md",
        "benches/test_medium.md",
        "benches/test_large.md",
    ];

    for file in files {
        let content =
            fs::read_to_string(file).unwrap_or_else(|_| panic!("Failed to read {}", file));
        divan::black_box(markdown_parser().parse(&content));
    }
}

#[divan::bench(min_time = 2)]
fn old_parse_multiple_files_concurrently() {
    let files = vec![
        "benches/test_small.md",
        "benches/test_medium.md",
        "benches/test_large.md",
    ];

    let handles: Vec<_> = files
        .into_iter()
        .map(|file| {
            let file = file.to_string();
            std::thread::spawn(move || {
                let content = fs::read_to_string(file).expect("Failed to read file");
                divan::black_box(markdown_parser().parse(&content));
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}

// New parser benchmarks

#[divan::bench(min_time = 2)]
fn new_parse_small_markdown_file() {
    let content = fs::read_to_string("benches/test_small.md").unwrap();

    divan::black_box(NewParser::new(&content).parse());
}

#[divan::bench(min_time = 2)]
fn new_parse_medium_markdown_file() {
    let content = fs::read_to_string("benches/test_medium.md").unwrap();

    divan::black_box(NewParser::new(&content).parse());
}

#[divan::bench(min_time = 2)]
fn new_parse_large_markdown_file() {
    let content = fs::read_to_string("benches/test_large.md").unwrap();

    divan::black_box(NewParser::new(&content).parse());
}

#[divan::bench(min_time = 2)]
fn new_parse_multiple_files_sequentially() {
    let files = vec![
        "benches/test_small.md",
        "benches/test_medium.md",
        "benches/test_large.md",
    ];

    for file in files {
        let content =
            fs::read_to_string(file).unwrap_or_else(|_| panic!("Failed to read {}", file));
        divan::black_box(NewParser::new(&content).parse());
    }
}

#[divan::bench(min_time = 2)]
fn new_parse_multiple_files_concurrently() {
    let files = vec![
        "benches/test_small.md",
        "benches/test_medium.md",
        "benches/test_large.md",
    ];

    let handles: Vec<_> = files
        .into_iter()
        .map(|file| {
            let file = file.to_string();
            std::thread::spawn(move || {
                let content = fs::read_to_string(file).expect("Failed to read file");
                divan::black_box(NewParser::new(&content).parse());
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}
