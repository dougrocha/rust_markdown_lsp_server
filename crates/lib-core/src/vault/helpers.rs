use std::path::Path;

use crate::vault::Vault;

use miette::Result;

/// General link struct to hold the target and header of a markdown link
pub struct LinkTarget<'a> {
    pub path: &'a str,
    pub header: Option<&'a str>,
}

// This will be a copy from lib-lsp get_content function, this will
// be the new function to use since it will not rely on server state
pub fn get_content(
    vault: &Vault,
    source_doc_uri: impl AsRef<Path>,
    link: LinkTarget,
) -> Result<String> {
    Ok(String::from(""))
}
