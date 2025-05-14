use std::path::PathBuf;

use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct URI(pub String);

impl Serialize for URI {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let prefixed_uri = format!("file://{}", self.0);
        serializer.serialize_str(&prefixed_uri)
    }
}

impl From<PathBuf> for URI {
    fn from(path: std::path::PathBuf) -> Self {
        URI(path.to_string_lossy().into_owned())
    }
}

impl<'de> Deserialize<'de> for URI {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let stripped = s.strip_prefix("file://");

        let uri_string = match stripped {
            Some(stripped_str) => stripped_str.to_string(),
            // Keep the original string if "file://" is not present
            None => s,
        };

        Ok(URI(uri_string))
    }
}

impl URI {
    pub fn to_path_buf(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(&self.0)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl std::fmt::Display for URI {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for URI {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
