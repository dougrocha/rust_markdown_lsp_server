use gen_lsp_types::Uri;
use std::borrow::Cow;
use std::path::{Path, PathBuf};

#[cfg(not(windows))]
pub use std::fs::canonicalize as strict_canonicalize;

/// On Windows, rewrites the wide path prefix `\\?\C:` to `C:`  
/// Source: https://stackoverflow.com/a/70970317
#[inline]
#[cfg(windows)]
fn strict_canonicalize<P: AsRef<Path>>(path: P) -> std::io::Result<PathBuf> {
    use std::io;

    fn impl_(path: PathBuf) -> std::io::Result<PathBuf> {
        let head = path
            .components()
            .next()
            .ok_or(io::Error::new(io::ErrorKind::Other, "empty path"))?;
        let disk_;
        let head = if let std::path::Component::Prefix(prefix) = head {
            if let std::path::Prefix::VerbatimDisk(disk) = prefix.kind() {
                disk_ = format!("{}:", disk as char);
                Path::new(&disk_).components().next().ok_or(io::Error::new(
                    io::ErrorKind::Other,
                    "failed to parse disk component",
                ))?
            } else {
                head
            }
        } else {
            head
        };
        Ok(std::iter::once(head)
            .chain(path.components().skip(1))
            .collect())
    }

    let canon = std::fs::canonicalize(path)?;
    impl_(canon)
}

/// Extract the path component from a URI string.
/// For file URIs, removes the `file://` prefix.
fn extract_path_from_uri(uri_str: &str) -> Option<&str> {
    uri_str.strip_prefix("file://")
}

mod sealed {
    pub trait Sealed {}
}

/// Provide methods to [`gen_lsp_types::Uri`] for converting to and from file paths.
pub trait UriExt: Sized + sealed::Sealed {
    /// Assuming the URL is in the `file` scheme or similar,
    /// convert its path to an absolute `std::path::Path`.
    ///
    /// **Note:** This does not actually check the URL's `scheme`, and may
    /// give nonsensical results for other schemes. It's the user's
    /// responsibility to check the URL's scheme before calling this.
    ///
    /// e.g. `Uri("file:///etc/passwd")` becomes `PathBuf("/etc/passwd")`
    fn to_file_path(&self) -> Option<Cow<'_, Path>>;

    /// Convert a file path to a [`gen_lsp_types::Uri`].
    ///
    /// Create a [`gen_lsp_types::Uri`] from a file path.
    ///
    /// Returns `None` if the file does not exist.
    fn from_file_path<A: AsRef<Path>>(path: A) -> Option<Self>;

    /// Get the file-stem directly from the Uri
    fn get_file_stem(&self) -> Option<String> {
        self.to_file_path().and_then(|path| {
            path.file_stem()
                .map(|os_str| os_str.to_string_lossy().into_owned())
        })
    }

    /// Get the file-name directly from the Uri
    fn get_file_name(&self) -> Option<String> {
        self.to_file_path().and_then(|path| {
            path.file_name()
                .map(|os_str| os_str.to_string_lossy().into_owned())
        })
    }

    /// Get the parent directory as a new Uri
    fn parent(&self) -> Option<Uri> {
        self.to_file_path()
            .and_then(|path| path.parent().map(|p| p.to_path_buf()))
            .and_then(Uri::from_file_path)
    }
}

impl sealed::Sealed for gen_lsp_types::Uri {}

impl UriExt for gen_lsp_types::Uri {
    fn to_file_path(&self) -> Option<Cow<'_, Path>> {
        let uri_str = self.as_ref();
        let path_str = extract_path_from_uri(uri_str)?;

        if cfg!(windows) {
            // On Windows, handle paths like /C:/path
            let path_str = if path_str.starts_with("/")
                && path_str.len() > 2
                && path_str.chars().nth(2) == Some(':')
            {
                // /C:/path -> C:/path
                &path_str[1..]
            } else {
                path_str
            };
            Some(Cow::Owned(PathBuf::from(path_str)))
        } else {
            Some(Cow::Owned(PathBuf::from(path_str)))
        }
    }

    fn from_file_path<A: AsRef<Path>>(path: A) -> Option<Self> {
        let path = path.as_ref();

        let fragment = if path.is_absolute() {
            Cow::Borrowed(path)
        } else {
            match strict_canonicalize(path) {
                Ok(path) => Cow::Owned(path),
                Err(_) => return None,
            }
        };

        let raw_uri = if cfg!(windows) {
            // we want to parse a triple-slash path for Windows paths
            // it's a shorthand for `file://localhost/C:/Windows` with the `localhost` omitted
            format!("file:///{}", fragment.to_string_lossy().replace("\\", "/"))
        } else {
            format!("file://{}", fragment.to_string_lossy())
        };

        // Uri is a type alias for fluent_uri::Uri<String> with the fluent-uri feature
        Uri::parse(raw_uri).ok()
    }
}

#[cfg(test)]
mod tests {
    #[cfg(windows)]
    use super::strict_canonicalize;
    use crate::uri::UriExt;
    use gen_lsp_types::Uri;
    use std::path::Path;

    #[test]
    #[cfg(windows)]
    fn test_idempotent_canonicalization() {
        let lhs = strict_canonicalize(Path::new(".")).unwrap();
        let rhs = strict_canonicalize(&lhs).unwrap();
        assert_eq!(lhs, rhs);
    }

    #[test]
    fn test_path_roundtrip_conversion() {
        // Use a hardcoded absolute path — no filesystem access needed.
        // Uri::from_file_path skips canonicalize for absolute paths.
        let src = Path::new("/some/absolute/path/note.md");
        let conv = Uri::from_file_path(src).unwrap();
        let roundtrip = conv.to_file_path().unwrap();
        assert_eq!(src, roundtrip, "conv={conv:?}");
    }

    #[test]
    #[cfg(windows)]
    fn test_windows_uri_roundtrip_conversion() {
        let uri = Uri::parse("file:///C:/Windows").unwrap();
        let path = uri.to_file_path().unwrap();
        assert_eq!(&path, Path::new("C:/Windows"), "uri={uri:?}");

        let conv = Uri::from_file_path(&path).unwrap();

        assert_eq!(uri, conv, "path={path:?}");
    }
}
