use crate::vault::Vault;

use miette::Result;

// This will be a copy from lib-lsp get_content function, this will
// be the new function to use since it will not rely on server state
pub fn get_content(vault: &Vault) -> Result<&str> {
    Ok("")
}
