use std::collections::HashMap;

use serde::Serialize;

use crate::{DocumentUri, uri::URI};

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<usize>,
}

impl OptionalVersionedTextDocumentIdentifier {
    pub fn new(uri: URI, version: Option<usize>) -> Self {
        OptionalVersionedTextDocumentIdentifier {
            text_document: TextDocumentIdentifier { uri },
            version,
        }
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentEdit {
    pub text_document: OptionalVersionedTextDocumentIdentifier,
    pub edits: Vec<TextEdit>,
}

#[derive(Serialize, Debug)]
pub struct CreateFileOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overwrite: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_if_exists: Option<bool>,
}

#[derive(Serialize, Debug)]
pub struct CreateFile {
    pub uri: DocumentUri,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<CreateFileOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotation_id: Option<ChangeAnnotationIdentifier>,
}

#[derive(Serialize, Debug)]
pub struct RenameFileOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overwrite: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_if_exists: Option<bool>,
}

#[derive(Serialize, Debug)]
pub struct RenameFile {
    pub old_uri: DocumentUri,
    pub new_uri: DocumentUri,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<RenameFileOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotation_id: Option<ChangeAnnotationIdentifier>,
}

#[derive(Serialize, Debug)]
pub struct DeleteFileOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recursive: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_if_not_exists: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotation_id: Option<ChangeAnnotationIdentifier>,
}

#[derive(Serialize, Debug)]
pub struct DeleteFile {
    pub uri: DocumentUri,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<DeleteFileOptions>,
}

#[derive(Serialize, Debug)]
#[serde(tag = "kind", rename_all = "lowercase")]
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

impl From<ResourceOp> for DocumentChangeOperation {
    fn from(op: ResourceOp) -> Self {
        DocumentChangeOperation::Op(op)
    }
}

impl From<TextDocumentEdit> for DocumentChangeOperation {
    fn from(edit: TextDocumentEdit) -> Self {
        DocumentChangeOperation::Edit(edit)
    }
}

pub type ChangeAnnotationIdentifier = String;

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum DocumentChanges {
    Edits(Vec<TextDocumentEdit>),
    Operations(Vec<DocumentChangeOperation>),
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceEdit {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changes: Option<HashMap<DocumentUri, Vec<TextEdit>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_changes: Option<DocumentChanges>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub change_annotations: Option<HashMap<ChangeAnnotationIdentifier, Vec<ChangeAnnotation>>>,
}
