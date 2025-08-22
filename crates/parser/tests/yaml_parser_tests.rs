use common::compare;
use parser::yaml::{yaml_parser, Frontmatter, Yaml};

mod common;

#[test]
fn test_yaml_parsing() {
    let input = r#"---
id: some-id
tags: one
---"#;

    let expected = Frontmatter(vec![
        ("id", Yaml::String("some-id")),
        ("tags", Yaml::String("one")),
    ]);

    compare(yaml_parser(), input, expected);
}

#[test]
fn test_yaml_parsing_with_list() {
    let input = r#"---
id: some-id
tags:
  - one
  - two
---"#;

    let expected = Frontmatter(vec![
        ("id", Yaml::String("some-id")),
        ("tags", Yaml::List(vec!["one", "two"])),
    ]);

    compare(yaml_parser(), input, expected);
}
