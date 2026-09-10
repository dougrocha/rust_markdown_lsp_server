use std::path::{Path, PathBuf};

use crate::config::LinkConfig;
use crate::document::{Document, Reference, index::Link};
use crate::path::{combine_and_normalize, slug::filename_slug, slug::header_slug};
use crate::vault::Vault;

/// Everything link resolution needs beyond the target string and the source path.
pub struct VaultContext<'a> {
    pub vault: &'a Vault,
    pub config: &'a LinkConfig,
    /// Every workspace root, deepest match wins for a given source path.
    pub workspace_roots: Vec<PathBuf>,
}

impl VaultContext<'_> {
    /// Return the deepest workspace root that is a prefix of `path`, or the only root.
    pub fn root_for(&self, path: &Path) -> Option<&Path> {
        match self.workspace_roots.as_slice() {
            [] => None,
            [only] => Some(only.as_path()),
            roots => roots
                .iter()
                .filter(|root| path.starts_with(root))
                .max_by_key(|root| root.as_os_str().len())
                .map(PathBuf::as_path),
        }
    }
}

/// A link target split into its file name and optional `#anchor`.
#[derive(Debug, PartialEq, Eq)]
pub struct LinkTarget<'a> {
    pub name: &'a str,
    pub anchor: Option<&'a str>,
}

/// Split a raw target string on the first `#`.
pub fn parse_target(raw: &str) -> LinkTarget<'_> {
    match raw.split_once('#') {
        Some((name, anchor)) => LinkTarget {
            name,
            anchor: Some(anchor),
        },
        None => LinkTarget { name: raw, anchor: None },
    }
}

/// Return true when the target points at an external resource.
pub fn is_external(target: &str) -> bool {
    let schemes = ["http://", "https://", "mailto:", "ftp://", "tel:"];
    schemes.iter().any(|scheme| target.starts_with(scheme))
}

/// Outcome of resolving a link target to a vault document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetResolution {
    File(PathBuf),
    Unresolved,
}

/// Resolve a link target to a document path, pure over `VaultContext`.
pub fn resolve(target: &str, source_path: &Path, cx: &VaultContext<'_>) -> TargetResolution {
    let name = parse_target(target).name;

    if is_path_syntax(name) {
        return resolve_as_path(name, source_path, cx.root_for(source_path));
    }

    if cx.config.enable_filename_resolution
        && let Some(path) = resolve_by_name(name, cx.vault)
    {
        return TargetResolution::File(path);
    }

    resolve_as_path(name, source_path, cx.root_for(source_path))
}

/// Check if a target uses path syntax rather than a bare file name.
fn is_path_syntax(target: &str) -> bool {
    target.starts_with('/')
        || target.starts_with("./")
        || target.starts_with("../")
        || target.contains('/')
        || target.contains('\\')
}

/// Resolve a target as an absolute or relative path.
fn resolve_as_path(
    target: &str,
    source_path: &Path,
    workspace_root: Option<&Path>,
) -> TargetResolution {
    if let Some(rest) = target.strip_prefix('/') {
        return match workspace_root {
            Some(root) => TargetResolution::File(root.join(rest)),
            None => TargetResolution::Unresolved,
        };
    }

    match combine_and_normalize(source_path, target) {
        Ok(path) => TargetResolution::File(path),
        Err(_) => TargetResolution::Unresolved,
    }
}

/// Resolve a bare file name through the index, with any trailing `.md` removed first.
fn resolve_by_name(target: &str, vault: &Vault) -> Option<PathBuf> {
    let stem = target.strip_suffix(".md").unwrap_or(target);
    let slug = filename_slug(stem);
    vault.index().resolve_name(&slug).map(Path::to_path_buf)
}

/// A single link somewhere in the vault, with the document it lives in.
pub struct LinkRef<'a> {
    pub document: &'a Document,
    pub link: &'a Link,
}

/// How a candidate link's `#anchor` must relate to the reference target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnchorMatch {
    /// Match the file regardless of any anchor.
    Any,
    /// Match only links written with no `#anchor`.
    None,
    /// Match only links whose `#anchor` slug equals this value.
    Slug(String),
}

/// The document location that references point at.
#[derive(Debug, Clone)]
pub struct ReferenceTarget {
    pub file: PathBuf,
    pub anchor: AnchorMatch,
}

impl ReferenceTarget {
    /// Build the query for the item under a cursor, or `None` for a footnote or an unresolved link.
    pub fn from_reference(
        reference: Reference<'_>,
        source_doc: &Document,
        cx: &VaultContext<'_>,
    ) -> Option<ReferenceTarget> {
        match reference {
            Reference::Link(link) => {
                let raw = link.target_str(&source_doc.source);
                let file = match resolve(&raw, &source_doc.path, cx) {
                    TargetResolution::File(path) => path,
                    TargetResolution::Unresolved => return None,
                };
                let anchor = match link.header_str(&source_doc.source) {
                    Some(header) => AnchorMatch::Slug(header.into_owned()),
                    None => AnchorMatch::None,
                };
                Some(ReferenceTarget { file, anchor })
            }
            Reference::Header(header) => Some(ReferenceTarget {
                file: source_doc.path.clone(),
                anchor: AnchorMatch::Slug(header.content_str(&source_doc.source).into_owned()),
            }),
            Reference::FootnoteRef(_) | Reference::FootnoteDef(_) => None,
        }
    }
}

/// Walk the vault once and yield every link that points at `target`, `O(D * L)` total.
pub fn find_link_references<'v, 'q>(
    target: &'q ReferenceTarget,
    cx: &'q VaultContext<'v>,
) -> impl Iterator<Item = LinkRef<'v>> + 'q {
    cx.vault.iter().flat_map(move |document| {
        document.links().filter_map(move |link| {
            link_points_at(link, document, target, cx).then_some(LinkRef { document, link })
        })
    })
}

/// Check if `link` in `document` points at `target`.
fn link_points_at(
    link: &Link,
    document: &Document,
    target: &ReferenceTarget,
    cx: &VaultContext<'_>,
) -> bool {
    let raw = link.target_str(&document.source);
    match resolve(&raw, &document.path, cx) {
        TargetResolution::File(path) if path == target.file => {}
        _ => return false,
    }

    match &target.anchor {
        AnchorMatch::Any => true,
        AnchorMatch::None => link.header_str(&document.source).is_none(),
        AnchorMatch::Slug(slug) => match link.header_str(&document.source) {
            Some(header) => header_slug(&header) == header_slug(slug),
            None => false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vault_with(files: &[(&str, &str)]) -> Vault {
        let mut vault = Vault::default();
        for (path, text) in files {
            vault
                .create_document(PathBuf::from(path), 0, text)
                .unwrap();
        }
        vault
    }

    fn config() -> LinkConfig {
        LinkConfig::default()
    }

    #[test]
    fn parses_anchor() {
        assert_eq!(
            parse_target("note#section"),
            LinkTarget {
                name: "note",
                anchor: Some("section")
            }
        );
        assert_eq!(
            parse_target("note"),
            LinkTarget {
                name: "note",
                anchor: None
            }
        );
    }

    #[test]
    fn resolves_bare_name_through_the_index() {
        let vault = vault_with(&[("/vault/My Note.md", "# Hi")]);
        let cfg = config();
        let cx = VaultContext {
            vault: &vault,
            config: &cfg,
            workspace_roots: vec![PathBuf::from("/vault")],
        };

        assert_eq!(
            resolve("my-note", Path::new("/vault/other.md"), &cx),
            TargetResolution::File(PathBuf::from("/vault/My Note.md"))
        );
    }

    #[test]
    fn resolves_relative_and_absolute_paths() {
        let vault = vault_with(&[("/vault/sub/target.md", "# T")]);
        let cfg = config();
        let cx = VaultContext {
            vault: &vault,
            config: &cfg,
            workspace_roots: vec![PathBuf::from("/vault")],
        };

        assert_eq!(
            resolve("./target.md", Path::new("/vault/sub/a.md"), &cx),
            TargetResolution::File(PathBuf::from("/vault/sub/target.md"))
        );
        assert_eq!(
            resolve("/sub/target.md", Path::new("/vault/sub/a.md"), &cx),
            TargetResolution::File(PathBuf::from("/vault/sub/target.md"))
        );
    }

    #[test]
    fn nested_roots_resolve_against_the_deepest_root() {
        let vault = vault_with(&[
            ("/vault/note.md", "# outer"),
            ("/vault/sub/note.md", "# inner"),
        ]);
        let cfg = config();
        let cx = VaultContext {
            vault: &vault,
            config: &cfg,
            workspace_roots: vec![PathBuf::from("/vault"), PathBuf::from("/vault/sub")],
        };

        assert_eq!(cx.root_for(Path::new("/vault/ref.md")), Some(Path::new("/vault")));
        assert_eq!(
            cx.root_for(Path::new("/vault/sub/ref.md")),
            Some(Path::new("/vault/sub"))
        );

        assert_eq!(
            resolve("/note.md", Path::new("/vault/ref.md"), &cx),
            TargetResolution::File(PathBuf::from("/vault/note.md"))
        );
        assert_eq!(
            resolve("/note.md", Path::new("/vault/sub/ref.md"), &cx),
            TargetResolution::File(PathBuf::from("/vault/sub/note.md"))
        );
    }

    #[test]
    fn find_link_references_resolves_each_candidate_against_its_own_root() {
        let vault = vault_with(&[
            ("/rootA/note.md", "# A"),
            ("/rootA/ref.md", "[x](/note.md)"),
            ("/rootB/note.md", "# B"),
            ("/rootB/ref.md", "[x](/note.md)"),
        ]);
        let cfg = config();
        let cx = VaultContext {
            vault: &vault,
            config: &cfg,
            workspace_roots: vec![PathBuf::from("/rootA"), PathBuf::from("/rootB")],
        };

        let target_a = ReferenceTarget {
            file: PathBuf::from("/rootA/note.md"),
            anchor: AnchorMatch::Any,
        };
        let hits: Vec<_> = find_link_references(&target_a, &cx)
            .map(|link_ref| link_ref.document.path.clone())
            .collect();
        assert_eq!(hits, vec![PathBuf::from("/rootA/ref.md")]);
    }

    #[test]
    fn absolute_path_without_root_is_unresolved() {
        let vault = vault_with(&[]);
        let cfg = config();
        let cx = VaultContext {
            vault: &vault,
            config: &cfg,
            workspace_roots: Vec::new(),
        };

        assert_eq!(
            resolve("/x.md", Path::new("/vault/a.md"), &cx),
            TargetResolution::Unresolved
        );
    }

    #[test]
    fn is_external_detects_schemes() {
        assert!(is_external("https://example.com"));
        assert!(is_external("mailto:a@b.com"));
        assert!(!is_external("./note.md"));
        assert!(!is_external("note"));
    }

    #[test]
    fn find_link_references_matches_wikilinks_and_inline_links() {
        let vault = vault_with(&[
            ("/vault/target.md", "# Section\n\nbody"),
            ("/vault/a.md", "[l](./target.md)"),
            ("/vault/b.md", "[[target]]"),
            ("/vault/c.md", "[[target#Section]]"),
        ]);
        let cfg = config();
        let cx = VaultContext {
            vault: &vault,
            config: &cfg,
            workspace_roots: vec![PathBuf::from("/vault")],
        };

        let any = ReferenceTarget {
            file: PathBuf::from("/vault/target.md"),
            anchor: AnchorMatch::Any,
        };
        assert_eq!(find_link_references(&any, &cx).count(), 3);

        let no_anchor = ReferenceTarget {
            file: PathBuf::from("/vault/target.md"),
            anchor: AnchorMatch::None,
        };
        assert_eq!(find_link_references(&no_anchor, &cx).count(), 2);

        let slug = ReferenceTarget {
            file: PathBuf::from("/vault/target.md"),
            anchor: AnchorMatch::Slug("Section".to_string()),
        };
        assert_eq!(find_link_references(&slug, &cx).count(), 1);
    }
}
