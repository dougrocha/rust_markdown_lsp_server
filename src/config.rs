use miette::{IntoDiagnostic, Result, WrapErr};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Configuration structure for the Rust Markdown LSP
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// LSP server settings
    pub server: ServerConfig,
    /// Markdown parsing settings
    pub markdown: MarkdownConfig,
    /// Diagnostics settings
    pub diagnostics: DiagnosticsConfig,
}

/// Server-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerConfig {
    /// Maximum number of files to process
    pub max_files: Option<usize>,
    /// Enable verbose logging
    pub verbose: bool,
}

/// Markdown parsing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkdownConfig {
    /// Enable frontmatter parsing
    pub enable_frontmatter: bool,
    /// Enable link validation
    pub validate_links: bool,
}

impl Default for MarkdownConfig {
    fn default() -> Self {
        Self {
            enable_frontmatter: true,
            validate_links: true,
        }
    }
}

/// Diagnostics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsConfig {
    /// Enable diagnostics for broken links
    pub enable_broken_links: bool,
    /// Enable diagnostics for missing frontmatter
    pub enable_missing_frontmatter: bool,
}

impl Default for DiagnosticsConfig {
    fn default() -> Self {
        Self {
            enable_broken_links: true,
            enable_missing_frontmatter: false,
        }
    }
}

impl Config {
    /// Load configuration from a TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let contents = fs::read_to_string(&path)
            .into_diagnostic()
            .with_context(|| format!("Failed to read config file: {:?}", path.as_ref()))?;

        let config: Config = toml::from_str(&contents)
            .into_diagnostic()
            .with_context(|| format!("Failed to parse config file: {:?}", path.as_ref()))?;

        Ok(config)
    }

    /// Load configuration from a TOML file, or return default if file doesn't exist
    pub fn from_file_or_default<P: AsRef<Path>>(path: P) -> Self {
        match Self::from_file(&path) {
            Ok(config) => config,
            Err(_) => {
                log::info!(
                    "Config file not found or invalid, using defaults: {:?}",
                    path.as_ref()
                );
                Self::default()
            }
        }
    }
}
