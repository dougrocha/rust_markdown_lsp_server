use lsp_types::{Position, Range as LspRange, Uri};

use crate::UriExt;

#[derive(Debug, Clone, PartialEq)]
pub struct Reference {
    pub kind: ReferenceKind,
    pub range: LspRange,
}

impl Reference {
    pub fn contains_position(&self, position: Position) -> bool {
        if position.line < self.range.start.line || position.line > self.range.end.line {
            return false;
        }

        if position.line == self.range.start.line && position.character < self.range.start.character
        {
            return false;
        }

        if position.line == self.range.end.line && position.character >= self.range.end.character {
            return false;
        }

        true
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReferenceKind {
    Header {
        level: usize,
        content: String,
    },
    Link {
        target: String,
        alt_text: String,
        title: Option<String>,
        header: Option<TargetHeader>,
    },
    WikiLink {
        target: String,
        alias: Option<String>,
        header: Option<TargetHeader>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct TargetHeader {
    pub level: usize,
    pub content: String,
}

impl ReferenceKind {
    /// Check if this reference targets a specific file
    pub fn targets_file(&self, file_uri: &Uri) -> bool {
        match self {
            ReferenceKind::Link { target, .. } => {
                // Handle relative paths and absolute paths
                let Some(file_path) = file_uri.to_file_path() else {
                    return false;
                };

                if let Some(file_name) = file_path.file_name() {
                    // Check if target matches filename (with or without extension)
                    target.ends_with(file_name.to_string_lossy().as_ref())
                        || target.ends_with(&file_name.to_string_lossy().trim_end_matches(".md"))
                } else {
                    false
                }
            }
            ReferenceKind::WikiLink { target, .. } => {
                // Wiki links typically use just the filename without extension
                let Some(file_path) = file_uri.to_file_path() else {
                    return false;
                };

                if let Some(file_name) = file_path.file_stem() {
                    target == file_name.to_string_lossy().as_ref()
                } else {
                    false
                }
            }
            ReferenceKind::Header { .. } => false,
        }
    }

    /// Check if this reference targets a specific header in a file
    pub fn targets_header(&self, file_uri: &Uri, header_content: &str) -> bool {
        match self {
            ReferenceKind::Link {
                header: Some(target_header),
                ..
            } => self.targets_file(file_uri) && target_header.content == header_content,
            ReferenceKind::WikiLink {
                header: Some(target_header),
                ..
            } => self.targets_file_wiki(file_uri) && target_header.content == header_content,
            _ => false,
        }
    }

    /// Check if this wiki link targets a specific file (helper method)
    fn targets_file_wiki(&self, file_uri: &Uri) -> bool {
        let ReferenceKind::WikiLink { target, .. } = self else {
            return false;
        };

        let Some(file_path) = file_uri.to_file_path() else {
            return false;
        };

        if let Some(file_name) = file_path.file_stem() {
            target == file_name.to_string_lossy().as_ref()
        } else {
            false
        }
    }

    /// Get the target identifier for matching purposes
    pub fn get_target_identifier(&self) -> Option<String> {
        match self {
            ReferenceKind::Header { content, .. } => Some(content.clone()),
            ReferenceKind::Link { target, .. } => Some(target.clone()),
            ReferenceKind::WikiLink { target, .. } => Some(target.clone()),
        }
    }
}
