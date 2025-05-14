pub mod code_action;
pub mod did_change;
pub mod did_open;
pub mod goto_definition;
pub mod hover;
pub mod initialize;
pub mod server;
pub mod workspace;

mod helpers;

use serde::{Deserialize, Serialize};

use crate::document::DocumentUri;

#[derive(Deserialize, Serialize, Debug)]
pub struct Range {
    /// The range's start position.
    pub start: Position,
    /// The range's end position.
    pub end: Position,
}

impl Range {
    /// If the selection is a range, the start and end are not equal
    pub fn is_range(&self) -> bool {
        self.start.line != self.end.line
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentItem {
    uri: DocumentUri,
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

#[derive(Serialize, Deserialize, Debug)]
pub struct TextDocumentIdentifier {
    uri: DocumentUri,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone, Copy)]
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
#[serde(rename_all = "camelCase")]
pub struct TextDocumentPositionParams {
    text_document: TextDocumentIdentifier,
    position: Position,
}

#[derive(Deserialize, Debug)]
pub struct FullTextDocumentContentChange {
    pub text: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct IncrementalTextDocumentContentChange {
    pub range: Range,
    pub range_length: Option<u32>,
    pub text: String,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum TextDocumentContentChangeEvent {
    Full(FullTextDocumentContentChange),
    Incremental(IncrementalTextDocumentContentChange),
}

#[derive(Deserialize, Debug)]
pub struct Diagnostic {
    /// The range at which the message applies.
    range: Range,
    /// The diagnostic's message.
    message: String,
}

#[derive(Serialize, Debug)]
pub struct Command {
    /// Title of the command, like `save`.
    title: String,
    /// The identifier of the actual command handler.
    command: String,
    /// Arguments that the command handler should be invoked with.
    arguments: Option<Vec<serde_json::Value>>,
}
