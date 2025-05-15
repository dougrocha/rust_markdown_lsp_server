use std::{
    ops::Range,
    path::{Path, PathBuf},
};

use lsp_types::uri::URI;

#[derive(Debug)]
pub enum Reference {
    // Header of a file
    Header {
        level: usize,
        content: String,
        span: Range<usize>,
    },
    // Tag ID
    Tag(URI),
    Link(LinkData),
    WikiLink(LinkData),
    Footnote,
}

// Find a way to distinguish between multiple types of links
// Internal, External, to other hearders, maybe ids?
#[derive(Debug, Clone, PartialEq)]
pub struct LinkData {
    pub source: URI,
    pub target: URI,
    pub span: Range<usize>,
    pub title: Option<String>,
    pub header: Option<LinkHeader>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinkHeader {
    pub level: usize,
    pub content: String,
}

pub fn combine_uri_and_relative_path(link_data: &LinkData) -> Option<PathBuf> {
    let source_dir = Path::new(link_data.source.as_str()).parent()?;
    Some(
        source_dir
            .join(link_data.target.as_str())
            .canonicalize()
            .unwrap_or(source_dir.to_path_buf()),
    )
}
