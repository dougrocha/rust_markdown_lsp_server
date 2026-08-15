#[derive(Debug, Clone, PartialEq)]
pub enum Yaml<'a> {
    String(&'a str),
    List(Vec<&'a str>),
}

type KeyValue<'a> = (&'a str, Yaml<'a>);

#[derive(Debug, Clone, PartialEq)]
pub struct Frontmatter<'a>(pub Vec<KeyValue<'a>>);

impl<'a> Frontmatter<'a> {
    pub fn get(&self, key: &str) -> Option<&Yaml<'a>> {
        self.0.iter().find(|(k, _)| *k == key).map(|(_, v)| v)
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Yaml<'a>> {
        self.0.iter_mut().find(|(k, _)| *k == key).map(|(_, v)| v)
    }
}

fn unquote(value: &str) -> &str {
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

/// Parses a full frontmatter block, including the leading and trailing `---`
/// delimiter lines. Only a flat subset of YAML is supported: `key: value`
/// pairs and `key:` followed by an indented `- item` list.
/// Returns `None` if `src` is not a well-formed frontmatter block.
pub fn parse_frontmatter(src: &str) -> Option<Frontmatter<'_>> {
    let mut lines = src.lines();

    if lines.next()?.trim() != "---" {
        return None;
    }

    let mut entries: Vec<KeyValue<'_>> = Vec::new();
    let mut pending_key: Option<&str> = None;
    let mut pending_list: Vec<&str> = Vec::new();

    for line in lines {
        if line.trim() == "---" {
            if let Some(key) = pending_key.take() {
                entries.push((key, Yaml::List(pending_list)));
            }
            return Some(Frontmatter(entries));
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("- ") {
            if pending_key.is_none() {
                return None; // list item without a preceding key
            }
            pending_list.push(unquote(rest.trim()));
            continue;
        }

        if let Some(key) = pending_key.take() {
            entries.push((key, Yaml::List(std::mem::take(&mut pending_list))));
        }

        let colon = trimmed.find(':')?;
        let key = trimmed[..colon].trim();
        if key.is_empty() {
            return None;
        }
        let value = trimmed[colon + 1..].trim();

        if value.is_empty() {
            pending_key = Some(key);
        } else {
            entries.push((key, Yaml::String(unquote(value))));
        }
    }

    None // no closing delimiter found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_key_values() {
        let input = "---\nid: some-id\ntags: one\n---";
        let result = parse_frontmatter(input).unwrap();
        assert_eq!(
            result,
            Frontmatter(vec![
                ("id", Yaml::String("some-id")),
                ("tags", Yaml::String("one")),
            ])
        );
    }

    #[test]
    fn parses_indented_list() {
        let input = "---\nid: some-id\ntags:\n  - one\n  - two\n---";
        let result = parse_frontmatter(input).unwrap();
        assert_eq!(
            result,
            Frontmatter(vec![
                ("id", Yaml::String("some-id")),
                ("tags", Yaml::List(vec!["one", "two"])),
            ])
        );
    }

    #[test]
    fn parses_quoted_string() {
        let input = "---\ntitle: \"Hello: World\"\n---";
        let result = parse_frontmatter(input).unwrap();
        assert_eq!(result.get("title"), Some(&Yaml::String("Hello: World")));
    }

    #[test]
    fn trailing_list_is_flushed_at_closing_delimiter() {
        let input = "---\ntags:\n  - one\n  - two\n---";
        let result = parse_frontmatter(input).unwrap();
        assert_eq!(result.get("tags"), Some(&Yaml::List(vec!["one", "two"])));
    }

    #[test]
    fn missing_closing_delimiter_returns_none() {
        let input = "---\nid: some-id\n";
        assert_eq!(parse_frontmatter(input), None);
    }

    #[test]
    fn missing_opening_delimiter_returns_none() {
        let input = "id: some-id\n---";
        assert_eq!(parse_frontmatter(input), None);
    }
}
