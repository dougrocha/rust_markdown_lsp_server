use lsp_types::{Position, Range as LspRange};

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
        /// The level of the header - H1, H2, H3
        level: usize,
        /// The content of the header
        content: String,
    },
    Link {
        /// The target URL/file path
        target: String,
        alt_text: String,
        title: Option<String>,
        /// Specific header in another markdown file
        header: Option<String>,
    },
    WikiLink {
        /// The target URL/file path
        target: String,
        alias: Option<String>,
        /// Specific header in another markdown file
        header: Option<String>,
    },
}

impl ReferenceKind {
    pub fn is_link(&self) -> bool {
        matches!(
            self,
            ReferenceKind::Link { .. } | ReferenceKind::WikiLink { .. }
        )
    }

    /// Get the target from a link
    pub fn get_target(&self) -> Option<&str> {
        match self {
            ReferenceKind::Link { target, .. } | ReferenceKind::WikiLink { target, .. } => {
                Some(target.as_str())
            }
            _ => None,
        }
    }

    pub fn get_link_header(&self) -> Option<&str> {
        match self {
            ReferenceKind::Link { header, .. } | ReferenceKind::WikiLink { header, .. } => {
                header.as_deref()
            }
            _ => None,
        }
    }

    pub fn get_content(&self) -> Option<&str> {
        match self {
            ReferenceKind::Header { content, .. } => Some(content.as_str()),
            _ => None,
        }
    }

    pub fn get_level(&self) -> Option<usize> {
        match self {
            ReferenceKind::Header { level, .. } => Some(*level),
            _ => None,
        }
    }

    pub fn get_alias(&self) -> Option<&str> {
        match self {
            ReferenceKind::WikiLink { alias, .. } => alias.as_deref(),
            _ => None,
        }
    }
}
