pub mod did_change;
pub mod did_open;
pub mod hover;
pub mod initialize;

use serde::{Deserialize, Deserializer, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct Range {
    /// The range's start position.
    start: Position,
    /// The range's end position.
    end: Position,
}

impl Range {
    fn from_span(src: &str, span: std::ops::Range<usize>) -> Self {
        let start_line_pos = str_indices::lines::from_byte_idx(src, span.start);
        let end_line_pos = str_indices::lines::from_byte_idx(src, span.end);
        let start_char = span.start - str_indices::lines::to_byte_idx(src, start_line_pos);
        let end_char = span.end - str_indices::lines::to_byte_idx(src, end_line_pos);

        Range {
            start: Position::new(start_line_pos, start_char),
            end: Position::new(end_line_pos, end_char),
        }
    }
}

#[derive(Debug, Clone)]
pub struct URI(pub String);

impl<'de> Deserialize<'de> for URI {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let stripped = s.strip_prefix("file://");

        let uri_string = match stripped {
            Some(stripped_str) => stripped_str.to_string(),
            // Keep the original string if "file://" is not present
            None => s,
        };

        Ok(URI(uri_string))
    }
}

impl URI {
    pub fn to_path_buf(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(&self.0)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

pub type DocumentUri = URI;

#[derive(Deserialize, Debug)]
pub struct TextDocumentItem {
    uri: DocumentUri,
    #[serde(rename = "languageId")]
    language_id: String,
    version: usize,
    text: String,
}

#[derive(Deserialize, Debug)]
pub struct VersionedTextDocumentIdentifier {
    #[serde(flatten)]
    pub text_document_identifier: TextDocumentIdentifier,
    pub version: usize,
}

#[derive(Deserialize, Debug)]
pub struct TextDocumentIdentifier {
    uri: DocumentUri,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct Position {
    pub line: usize,
    pub character: usize,
}

impl Position {
    pub fn new(line: usize, character: usize) -> Self {
        Self { line, character }
    }
}

#[derive(Deserialize, Debug)]
pub struct TextDocumentPositionParams {
    #[serde(rename = "textDocument")]
    text_document: TextDocumentIdentifier,
    position: Position,
}

#[derive(Deserialize, Debug)]
pub struct FullTextDocumentContentChange {
    pub text: String,
}

#[derive(Deserialize, Debug)]
pub struct IncrementalTextDocumentContentChange {
    pub range: Range,
    #[serde(rename = "rangeLength")]
    pub range_length: Option<u32>,
    pub text: String,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum TextDocumentContentChangeEvent {
    Full(FullTextDocumentContentChange),
    Incremental(IncrementalTextDocumentContentChange),
}
