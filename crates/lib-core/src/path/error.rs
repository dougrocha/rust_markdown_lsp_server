use std::path::PathBuf;

use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum PathError {
    #[error("Directory has no parent: {0}")]
    #[diagnostic(code(path::no_parent))]
    NoParent(PathBuf),

    #[error("IO error during path manipulation: {0}")]
    #[diagnostic(code(path::io_error))]
    Io(#[from] std::io::Error),

    #[error("Could not determine relative path from {base} to {target}")]
    #[diagnostic(code(path::diff_error))]
    RelativeDiff { base: PathBuf, target: PathBuf },
}
