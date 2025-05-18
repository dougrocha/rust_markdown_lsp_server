use miette::{bail, Result};

use std::{ops::Range, path::Path, str::FromStr};

use lsp_types::Uri;

#[derive(Debug)]
pub enum Reference {
    // Header of a file
    Header {
        level: usize,
        content: String,
        span: Range<usize>,
    },
    // Tag ID
    Tag(Uri),
    Link(LinkData),
    WikiLink(LinkData),
    Footnote,
}

// Find a way to distinguish between multiple types of links
// Internal, External, to other hearders, maybe ids?
#[derive(Debug, Clone, PartialEq)]
pub struct LinkData {
    pub source: Uri,
    pub target: Uri,
    pub span: Range<usize>,
    pub title: Option<String>,
    pub header: Option<LinkHeader>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinkHeader {
    pub level: usize,
    pub content: String,
}

pub fn combine_uri_and_relative_path(source: &Uri, target: &Uri) -> Result<Uri> {
    let source_dir = Path::new(source.path().as_str())
        .parent()
        .expect("Source directory cannot be None");

    let target_path = target.as_str();

    let combined_path = source_dir.join(target_path);

    let path = combined_path.canonicalize();

    Ok(Uri::from_str(path.unwrap().as_os_str().to_str().unwrap()).unwrap())
}
