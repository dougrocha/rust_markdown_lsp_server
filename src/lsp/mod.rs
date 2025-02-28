pub mod did_change;
pub mod did_open;
pub mod hover;
pub mod initialize;

use std::ops::Range;

use serde::Deserialize;

pub type DocumentUri = String;

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

#[derive(Deserialize, Debug)]
pub struct Position {
    line: usize,
    character: usize,
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
    pub range: Range<usize>,
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
