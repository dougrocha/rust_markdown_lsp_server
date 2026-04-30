mod error;
pub mod slug;

use std::path::{Path, PathBuf};

use error::PathError;
use miette::{Result, miette};
use path_clean::PathClean;

use crate::document::references::Reference;

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

/// Extract filename without extension from a path.
/// Example: /path/to/note.md -> Some("note")
pub fn extract_filename_stem(path: &Path) -> Option<String> {
    let filename = path.file_stem()?;
    Some(filename.to_string_lossy().into_owned())
}

/// Extract full filename with extension from a path.
/// Example: /path/to/note.md -> Some("note.md")
pub fn extract_filename(path: &Path) -> Option<String> {
    let filename = path.file_name()?;
    Some(filename.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_relative_path_same_dir() {
        let source = "/project/notes/a.md";
        let target = "/project/notes/b.md";

        let result = find_relative_path(source, target).unwrap();
        assert_eq!(result, "./b.md");
    }

    #[test]
    fn test_find_relative_path_nested_child() {
        let source = "/project/notes/a.md";
        let target = "/project/notes/subdir/c.md";

        let result = find_relative_path(source, target).unwrap();
        assert_eq!(result, "./subdir/c.md");
    }

    #[test]
    fn test_find_relative_path_parent_dir() {
        let source = "/project/notes/subdir/a.md";
        let target = "/project/notes/root.md";

        let result = find_relative_path(source, target).unwrap();
        assert_eq!(result, "../root.md");
    }

    #[test]
    fn test_find_relative_path_exact_same_file() {
        let source = "/project/notes/a.md";
        let target = "/project/notes/a.md";

        let result = find_relative_path(source, target).unwrap();
        assert_eq!(result, "./a.md");
    }

    #[test]
    fn test_extract_filename_stem() {
        let path = Path::new("/path/to/note.md");
        assert_eq!(extract_filename_stem(path), Some("note".to_string()));

        let path = Path::new("/my-note.md");
        assert_eq!(extract_filename_stem(path), Some("my-note".to_string()));
    }
}
