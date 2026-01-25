use chumsky::Parser;
use parser::markdown::header_parser;

fn main() {
    // Test header with newline
    let input1 = "# Header Text\n";
    let result1 = header_parser().parse(input1).into_output_errors();
    println!("WITH NEWLINE: '{}'", input1.replace("\n", "\\n"));
    println!("  Parsed: {}", result1.0.is_some());
    println!("  Errors: {}", result1.1.len());
    if !result1.1.is_empty() {
        println!("  Error: {}", result1.1[0].reason());
    }
    
    // Test header without newline
    let input2 = "# Header Text";
    let result2 = header_parser().parse(input2).into_output_errors();
    println!("\nWITHOUT NEWLINE: '{}'", input2);
    println!("  Parsed: {}", result2.0.is_some());
    println!("  Errors: {}", result2.1.len());
    if let Some(header) = result2.0 {
        println!("  Result: {:?}", header);
    }
}
