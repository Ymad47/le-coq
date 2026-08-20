//! Centralized configuration
//! All paths and environment-specific settings live here.

use std::path::PathBuf;

/// Project root — overridable via LE_COQ_ROOT env var
pub fn le_coq_root() -> PathBuf {
    std::env::var("LE_COQ_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/home/yaya/le-coq"))
}

/// Le Chasseur output directory
pub fn le_chasseur_output_dir() -> PathBuf {
    std::env::var("LE_CHASSEUR_OUTPUT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| le_coq_root().join("le-chasseur/output"))
}

/// Python plugins directory
pub fn plugins_dir() -> PathBuf {
    std::env::var("LE_COQ_PLUGINS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| le_coq_root().join("plugins"))
}

/// Python plugins venv site-packages
pub fn plugins_site_packages() -> PathBuf {
    std::env::var("LE_COQ_PLUGINS_VENV")
        .map(PathBuf::from)
        .unwrap_or_else(|_| plugins_dir().join(".venv/lib/python3.11/site-packages"))
}
