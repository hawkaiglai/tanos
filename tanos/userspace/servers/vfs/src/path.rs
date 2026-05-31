//! Path validation and normalization for the VFS server

use alloc::string::String;
use alloc::vec::Vec;
use crate::lib_extensions::{Error, Result};

/// Validated filesystem path
pub struct Path {
    normalized: String,
}

impl Path {
    /// Create a new validated path
    pub fn new(path_str: &str) -> Result<Self> {
        if path_str.is_empty() {
            return Err(Error::InvalidPath);
        }

        // Must be absolute
        if !path_str.starts_with('/') {
            return Err(Error::InvalidPath);
        }

        // Check for null bytes
        if path_str.bytes().any(|b| b == 0) {
            return Err(Error::InvalidPath);
        }

        // Normalize: resolve . and ..
        let normalized = Self::normalize(path_str);
        Ok(Self { normalized })
    }

    pub fn as_str(&self) -> &str {
        &self.normalized
    }

    fn normalize(path: &str) -> String {
        let mut components: Vec<&str> = Vec::new();

        for component in path.split('/') {
            match component {
                "" | "." => {}
                ".." => { components.pop(); }
                c => components.push(c),
            }
        }

        if components.is_empty() {
            String::from("/")
        } else {
            let mut result = String::new();
            for c in &components {
                result.push('/');
                result.push_str(c);
            }
            result
        }
    }
}
