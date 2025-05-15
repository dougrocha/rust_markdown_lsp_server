use serde::{Deserialize, Serialize};

use crate::{Command, Diagnostic, Range, TextDocumentIdentifier, workspace::WorkspaceEdit};

#[derive(Deserialize, Debug)]
pub struct CodeActionContext {
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CodeActionParams {
    pub text_document: TextDocumentIdentifier,
    pub range: Range,
    pub context: CodeActionContext,
}

#[derive(Serialize, Debug)]
pub enum CodeActionKind {
    #[serde(rename(serialize = "refactor"))]
    Refactor,
    #[serde(rename(serialize = "refactor.extract"))]
    RefactorExtract,
}

#[derive(Serialize, Debug)]
pub struct CodeAction {
    pub title: String,
    pub kind: Option<CodeActionKind>,
    pub edit: Option<WorkspaceEdit>,
}

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum CodeActionOrCommand {
    Command(Command),
    CodeAction(CodeAction),
}

pub type CodeActionResponse = Option<Vec<CodeActionOrCommand>>;
