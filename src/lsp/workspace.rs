use std::collections::HashMap;

use serde::Serialize;

use crate::document::DocumentUri;

use super::{Range, TextDocumentIdentifier};

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TextEdit {
    pub range: Range,
    pub new_text: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ChangeAnnotation {
    pub label: String,
    pub needs_confirmation: Option<bool>,
    pub description: Option<String>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OptionalVersionedTextDocumentIdentifier {
    #[serde(flatten)]
    pub text_document: TextDocumentIdentifier,
    pub version: Option<usize>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentEdit {
    pub text_document: OptionalVersionedTextDocumentIdentifier,
    pub edits: Vec<TextEdit>,
}

#[derive(Serialize, Debug)]
pub struct CreateFileOptions {
    pub overwrite: Option<bool>,
    pub ignore_if_exists: Option<bool>,
}

#[derive(Serialize, Debug)]
pub struct CreateFile {
    pub uri: DocumentUri,
    pub options: Option<CreateFileOptions>,
    pub annotation_id: Option<ChangeAnnotationIdentifier>,
}

#[derive(Serialize, Debug)]
pub struct RenameFileOptions {
    pub overwrite: Option<bool>,
    pub ignore_if_exists: Option<bool>,
}

#[derive(Serialize, Debug)]
pub struct RenameFile {
    pub old_uri: DocumentUri,
    pub new_uri: DocumentUri,
    pub options: Option<RenameFileOptions>,
    pub annotation_id: Option<ChangeAnnotationIdentifier>,
}

#[derive(Serialize, Debug)]
pub struct DeleteFileOptions {
    pub recursive: Option<bool>,
    pub ignore_if_not_exists: Option<bool>,
    pub annotation_id: Option<ChangeAnnotationIdentifier>,
}

#[derive(Serialize, Debug)]
pub struct DeleteFile {
    pub uri: DocumentUri,
    pub options: Option<DeleteFileOptions>,
}

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum ResourceOp {
    Create(CreateFile),
    Rename(RenameFile),
    Delete(DeleteFile),
}

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum DocumentChangeOperation {
    Op(ResourceOp),
    Edit(TextDocumentEdit),
}

pub type ChangeAnnotationIdentifier = String;

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum DocumentEdits {
    Edits(Vec<TextDocumentEdit>),
    Operations(Vec<DocumentChangeOperation>),
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceEdit {
    pub changes: Option<HashMap<DocumentUri, Vec<TextEdit>>>,
    pub document_changes: Option<Vec<DocumentEdits>>,
    pub change_annotations: Option<HashMap<ChangeAnnotationIdentifier, Vec<ChangeAnnotation>>>,
}
