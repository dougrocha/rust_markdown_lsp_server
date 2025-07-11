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
    /// Get the target identifier for matching purposes
    pub fn get_target_identifier(&self) -> Option<&str> {
        match self {
            ReferenceKind::Header { content, .. } => Some(content.as_str()),
            ReferenceKind::Link { target, .. } => Some(target.as_str()),
            ReferenceKind::WikiLink { target, .. } => Some(target.as_str()),
        }
    }
}
