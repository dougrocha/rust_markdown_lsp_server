use lsp_types::Uri;
use miette::{miette, Context, IntoDiagnostic, Result};

use crate::UriExt;

pub fn get_parent_path(uri: &Uri) -> Option<String> {
    let mut segments = uri.path().segments().collect::<Vec<_>>();

    segments.pop();

    Some(format!("/{}", segments.join("/")))
}

pub fn combine_and_normalize(source: &Uri, target: &Uri) -> Result<Uri> {
    let source = source
        .to_file_path()
        .expect("Failed to retrieve source directory");
    let parent = source
        .parent()
        .expect("Failed to retrieve parent directory");

    let target_path = target.as_str();

    let combined_path = parent.join(target_path);

    let path = combined_path
        .canonicalize()
        .into_diagnostic()
        .context("Failed to canonicalize the combined path")?;

    let new_path =
        Uri::from_file_path(path).ok_or_else(|| miette!("Failed to create URI from file path"))?;
    Ok(new_path)
}

pub fn find_relative_path(source: &Uri, target: &Uri) -> Result<String> {
    let source_path = source
        .to_file_path()
        .ok_or_else(|| miette!("Failed to convert source URI to file path"))?;

    let target_path = target
        .to_file_path()
        .ok_or_else(|| miette!("Failed to convert target URI to file path"))?;

    let base_dir = source_path.parent().ok_or_else(|| {
        miette::miette!(
            "Source path has no parent directory: {}",
            source_path.display()
        )
    })?;

    let relative_path_buf = pathdiff::diff_paths(&target_path, &base_dir).ok_or_else(|| {
        miette::miette!(
            "Could not determine relative path from '{}' to '{}'",
            base_dir.display(),
            target_path.display()
        )
    })?;

    let relative_path_str = relative_path_buf.to_string_lossy().into_owned();

    if relative_path_str.is_empty() {
        Ok("./".to_string())
    } else if !relative_path_str.starts_with('.') && !relative_path_buf.is_absolute() {
        // This heuristic adds "./" for direct children like "file.txt" or "subdir/file.txt"
        // It avoids adding "./" for "../" paths or absolute paths if they somehow sneaked through
        // (though diff_paths should produce relative paths).
        Ok(format!("./{}", relative_path_str))
    } else {
        Ok(relative_path_str)
    }
}
