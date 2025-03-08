use std::path::PathBuf;

use serde::{Deserialize, Deserializer, Serialize};

#[derive(Serialize, Debug, Clone, Eq, PartialEq)]
pub struct URI(pub String);

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
