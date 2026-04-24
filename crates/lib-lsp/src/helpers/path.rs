use lib_core::document::references::Reference;
use miette::{Result, miette};
use path_clean::PathClean;
use std::path::{Path, PathBuf};

/// Computes the relative path between a source file and a target file.
///
/// Will compute the difference between the source file's directory versus the target file.
pub fn relative_path(from_file: impl AsRef<Path>, to_file: impl AsRef<Path>) -> Result<String> {
    let from_file = from_file.as_ref();
    let to_file = to_file.as_ref();

    let from_parent = from_file
        .parent()
        .ok_or_else(|| miette!("Could not find parent path from 'from_file'"))?;

    let mut rel = pathdiff::diff_paths(to_file, from_parent)
        .ok_or(miette!("Could not compute diff"))?
        .to_string_lossy()
        .to_string();

    if !rel.starts_with('.') {
        rel = format!("./{}", rel);
    }

    Ok(rel)
}

/// Resolve a reference to its absolute filepath equivalent
pub fn resolve_reference_target(
    doc_path: impl AsRef<Path>,
    reference: &Reference,
) -> Result<PathBuf> {
    let doc_path = doc_path.as_ref();
    let base_path = doc_path.parent().ok_or(miette!("No parent"))?;
    let target_str = reference.kind.get_target().ok_or(miette!("No target"))?;
    Ok(base_path.join(target_str).clean())
}
