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
