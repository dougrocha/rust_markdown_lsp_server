mod error;

use std::path::Path;

use error::PathError;
use lsp_types::Uri;
use miette::Result;

use crate::uri::UriExt;

pub fn get_parent_path(uri: &Uri) -> Option<String> {
    let path = Path::new(uri.path().as_str());
    path.parent().map(|p| p.to_string_lossy().into_owned())
}

pub fn combine_and_normalize(source: &Uri, target: &str) -> Result<Uri, PathError> {
    let source_path = source
        .to_file_path()
        .ok_or_else(|| PathError::InvalidUri(source.to_string()))?;

    let parent_path = source_path
        .parent()
        .ok_or_else(|| PathError::NoParent(source_path.to_path_buf()))?;

    let combined_path = parent_path.join(target);

    let path = combined_path.canonicalize().map_err(PathError::Io)?;

    Uri::from_file_path(path).ok_or_else(|| PathError::InvalidUri(source.to_string()))
}

pub fn find_relative_path(source: &Uri, target: &Uri) -> Result<String, PathError> {
    let source_path = source
        .to_file_path()
        .ok_or_else(|| PathError::InvalidUri(source.to_string()))?;

    let target_path = target
        .to_file_path()
        .ok_or_else(|| PathError::InvalidUri(target.to_string()))?;

    let base_dir = source_path
        .parent()
        .ok_or_else(|| PathError::NoParent(source_path.to_path_buf()))?;

    let relative_path_buf =
        pathdiff::diff_paths(&target_path, base_dir).ok_or_else(|| PathError::RelativeDiff {
            base: base_dir.to_path_buf(),
            target: target_path.to_path_buf(),
        })?;

    let relative_path_str = relative_path_buf.to_string_lossy();

    // This heuristic adds "./" for direct children like "file.txt" or "subdir/file.txt"
    // It avoids adding "./" for "../" paths or absolute paths if they somehow sneaked through
    // (though diff_paths should produce relative paths).
    if relative_path_str.is_empty() {
        Ok("./".to_string())
    } else if !relative_path_str.starts_with('.') && !relative_path_buf.is_absolute() {
        Ok(format!("./{relative_path_str}"))
    } else {
        Ok(relative_path_str.into_owned())
    }
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
        let source = Uri::from_str("file:///project/notes/a.md").unwrap();
        let target = Uri::from_str("file:///project/notes/b.md").unwrap();

        let result = find_relative_path(&source, &target).unwrap();
        // Should trigger the "./" heuristic
        assert_eq!(result, "./b.md");
    }

    #[test]
    fn test_find_relative_path_nested_child() {
        let source = Uri::from_str("file:///project/notes/a.md").unwrap();
        let target = Uri::from_str("file:///project/notes/subdir/c.md").unwrap();

        let result = find_relative_path(&source, &target).unwrap();
        // Should trigger the "./" heuristic for the folder
        assert_eq!(result, "./subdir/c.md");
    }

    #[test]
    fn test_find_relative_path_parent_dir() {
        let source = Uri::from_str("file:///project/notes/subdir/a.md").unwrap();
        let target = Uri::from_str("file:///project/notes/root.md").unwrap();

        let result = find_relative_path(&source, &target).unwrap();
        // Should NOT trigger "./" because it starts with ".."
        assert_eq!(result, "../root.md");
    }

    #[test]
    fn test_find_relative_path_exact_same_file() {
        let source = Uri::from_str("file:///project/notes/a.md").unwrap();
        let target = Uri::from_str("file:///project/notes/a.md").unwrap();

        let result = find_relative_path(&source, &target).unwrap();
        // Should handle the empty string case
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
