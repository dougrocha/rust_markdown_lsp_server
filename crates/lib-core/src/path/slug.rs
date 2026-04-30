/// Normalizes a header string to a GFM-compatible anchor slug
pub fn header_slug(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let mut last_was_dash = false;

    for c in content.to_lowercase().chars() {
        if c.is_alphanumeric() {
            result.push(c);
            last_was_dash = false;
        } else if !last_was_dash {
            result.push('-');
            last_was_dash = true;
        }
    }

    result.trim_matches('-').to_string()
}

/// Normalizes a filename string for wikilink-to-file matching
pub fn filename_slug(file_str: &str) -> String {
    file_str
        .to_lowercase()
        .chars()
        .map(|c| match c {
            ' ' | '_' | '-' => '-',
            c => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_slug_basic() {
        assert_eq!(header_slug("Example Header"), "example-header");
        assert_eq!(header_slug("Example  Header"), "example-header");
        assert_eq!(header_slug("Example-Header"), "example-header");
        assert_eq!(header_slug("Example_Header"), "example-header");
        assert_eq!(header_slug("Example Header!"), "example-header");
    }

    #[test]
    fn test_header_slug_strips_special_chars() {
        assert_eq!(header_slug("Example & Header"), "example-header");
        assert_eq!(header_slug("Hello, World!"), "hello-world");
        assert_eq!(header_slug("C++ Notes"), "c-notes");
    }

    #[test]
    fn test_header_slug_trims_leading_trailing_dashes() {
        assert_eq!(header_slug("!Header"), "header");
        assert_eq!(header_slug("Header!"), "header");
        assert_eq!(header_slug("!Header!"), "header");
    }

    #[test]
    fn test_header_slug_hash_prefix() {
        let anchor = "#example-header";
        let stripped = anchor.strip_prefix('#').unwrap();
        assert_eq!(header_slug("Example Header"), stripped);
        assert_eq!(header_slug("Example Header"), header_slug(stripped));
    }

    #[test]
    fn test_header_slug_unicode_lowercase() {
        assert_eq!(header_slug("Ñoño"), "ñoño");
        assert_eq!(header_slug("Über Header"), "über-header");
    }

    #[test]
    fn test_filename_slug_basic() {
        assert_eq!(filename_slug("My Note"), "my-note");
        assert_eq!(filename_slug("my_note"), "my-note");
        assert_eq!(filename_slug("MY-NOTE"), "my-note");
        assert_eq!(filename_slug("My_Cool_Note"), "my-cool-note");
    }

    #[test]
    fn test_filename_slug_preserves_special_chars() {
        assert_eq!(filename_slug("My & Note"), "my-&-note");
        assert_eq!(filename_slug("Notes (2024)"), "notes-(2024)");
    }

    #[test]
    fn test_filename_slug_unicode_lowercase() {
        assert_eq!(filename_slug("Über Note"), "über-note");
    }

    #[test]
    fn test_header_and_filename_slug_differ_on_special_chars() {
        assert_eq!(header_slug("My & Header"), "my-header");
        assert_eq!(filename_slug("My & Note"), "my-&-note");
        assert_ne!(
            header_slug("My & Header"),
            filename_slug("My & Header"),
            "The two functions must produce different results for inputs with special chars"
        );
    }
}
