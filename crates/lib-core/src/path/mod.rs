mod error;

use std::path::{Path, PathBuf};

use error::PathError;
use lsp_types::Uri;
use miette::{Result, miette};
use path_clean::PathClean;

use crate::{document::references::Reference, uri::UriExt};

/// Resolve a reference to its absolute filepath equivalent
pub fn resolve_reference_target(
    source_path: impl AsRef<Path>,
    reference: &Reference,
) -> Result<PathBuf> {
    let target_str = reference.kind.get_target().ok_or(miette!("No target"))?;

    Ok(combine_and_normalize(source_path, target_str)?)
}

// TODO: Not sure if I really like this name
pub fn combine_and_normalize(
    source_path: impl AsRef<Path>,
    target_path: &str,
) -> Result<PathBuf, PathError> {
    let source_path = source_path.as_ref();

    let source_parent = source_path
        .parent()
        .ok_or_else(|| PathError::NoParent(source_path.to_path_buf()))?;

    Ok(source_parent.join(target_path).clean())
}

/// Computes the relative path between a source file and a target file.
///
/// Will compute the difference between the source file's directory versus the target file.
pub fn find_relative_path(
    source_path: impl AsRef<Path>,
    target_path: impl AsRef<Path>,
) -> Result<String, PathError> {
    let from_file = source_path.as_ref();
    let to_file = target_path.as_ref();

    let from_parent = from_file
        .parent()
        .ok_or_else(|| PathError::NoParent(from_file.to_path_buf()))?;

    let mut rel = pathdiff::diff_paths(to_file, from_parent)
        .ok_or_else(|| PathError::RelativeDiff {
            base: from_parent.to_path_buf(),
            target: to_file.to_path_buf(),
        })?
        .to_string_lossy()
        .to_string();

    if !rel.starts_with('.') {
        rel = format!("./{}", rel);
    }

    Ok(rel)
}

/// Extract filename without extension from URI
/// Example: file:///path/to/note.md -> Some("note")
pub fn extract_filename_stem(uri: &Uri) -> Option<String> {
    let path = uri.to_file_path()?;
    let filename = path.file_stem()?;
    Some(filename.to_string_lossy().into_owned())
}

/// Extract full filename with extension from URI
/// Example: file:///path/to/note.md -> Some("note.md")
pub fn extract_filename(uri: &Uri) -> Option<String> {
    let path = uri.to_file_path()?;
    let filename = path.file_name()?;
    Some(filename.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_find_relative_path_same_dir() {
        let source = "file:///project/notes/a.md";
        let target = "file:///project/notes/b.md";

        let result = find_relative_path(source, target).unwrap();
        assert_eq!(result, "./b.md");
    }

    #[test]
    fn test_find_relative_path_nested_child() {
        let source = "file:///project/notes/a.md";
        let target = "file:///project/notes/subdir/c.md";

        let result = find_relative_path(source, target).unwrap();
        assert_eq!(result, "./subdir/c.md");
    }

    #[test]
    fn test_find_relative_path_parent_dir() {
        let source = "file:///project/notes/subdir/a.md";
        let target = "file:///project/notes/root.md";

        let result = find_relative_path(source, target).unwrap();
        assert_eq!(result, "../root.md");
    }

    #[test]
    fn test_find_relative_path_exact_same_file() {
        let source = "file:///project/notes/a.md";
        let target = "file:///project/notes/a.md";

        let result = find_relative_path(source, target).unwrap();
        assert_eq!(result, "./a.md");
    }

    #[test]
    fn test_extract_filename_stem() {
        let uri = Uri::from_str("file:///path/to/note.md").unwrap();
        assert_eq!(extract_filename_stem(&uri), Some("note".to_string()));

        let uri = Uri::from_str("file:///my-note.md").unwrap();
        assert_eq!(extract_filename_stem(&uri), Some("my-note".to_string()));
    }
}
