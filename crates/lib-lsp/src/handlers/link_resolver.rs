use std::path::Path;

use gen_lsp_types::Uri;
use miette::{Result, miette};

use lib_core::{
    document::Document,
    resolver::{self, TargetResolution, VaultContext},
};

use crate::{ServerState, uri::UriExt};

/// Resolve a link target for `document` to a workspace `Uri`.
pub fn resolve_target_uri(lsp: &ServerState, document: &Document, target: &str) -> Result<Uri> {
    let cx = lsp.vault_context();
    resolve_in_context(target, &document.path, &cx)
}

/// Resolve a link target to a `Uri` using an already built `VaultContext`.
pub fn resolve_in_context(
    target: &str,
    source_path: &Path,
    cx: &VaultContext<'_>,
) -> Result<Uri> {
    match resolver::resolve(target, source_path, cx) {
        TargetResolution::File(path) => Uri::from_file_path(&path)
            .ok_or_else(|| miette!("Failed to build URI for resolved path: {:?}", path)),
        TargetResolution::Unresolved => {
            Err(miette!("Could not resolve link target: {target}"))
        }
    }
}
