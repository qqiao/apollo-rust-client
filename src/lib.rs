use std::fmt::Display;

use cache::Cache;
use config::Config;

pub mod cache;
pub mod config;

/// Different types of errors that can occur when using the client.
#[derive(Debug, thiserror::Error)]
pub enum Error {}

/// Apollo client.
pub struct Client {
    _config: Config,
}

/// Different types of environments.
pub enum Env {
    /// Development environment.
    Dev,
    /// Feature Acceptance Testing environment.
    Fat,
    /// User Acceptance Testing environment.
    Uat,
    /// Production environment.
    Pro,
}

impl Display for Env {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dev => write!(f, "DEV"),
            Self::Fat => write!(f, "FAT"),
            Self::Uat => write!(f, "UAT"),
            Self::Pro => write!(f, "PRO"),
        }
    }
}

impl Client {
    /// Create a new Apollo client.
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration for the Apollo client.
    ///
    /// # Returns
    ///
    /// A new Apollo client.
    pub fn new(config: Config) -> Self {
        Self { _config: config }
    }

    /// Get a cache for a given namespace.
    ///
    /// # Arguments
    ///
    /// * `name_space` - The namespace to get the cache for.
    ///
    /// # Returns
    ///
    /// A cache for the given namespace.
    pub fn namespace(&self, _name_space: &str) -> &Cache {
        todo!()
    }
}

#[cfg(test)]
mod tests {}
