pub mod did_change;
pub mod did_open;
pub mod goto_definition;
pub mod hover;
pub mod initialize;
pub mod server;

use serde::{Deserialize, Serialize};

use crate::document::DocumentUri;

#[derive(Deserialize, Serialize, Debug)]
pub struct Range {
    /// The range's start position.
    pub start: Position,
    /// The range's end position.
    pub end: Position,
}

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
