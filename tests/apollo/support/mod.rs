//! Support utilities for real Apollo integration tests.

#![cfg(not(target_arch = "wasm32"))]
#![allow(clippy::pedantic)]

use apollo_rust_client::client_config::{ClientConfig, ClientConfigBuilder};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static CACHE_COUNTER: AtomicUsize = AtomicUsize::new(1);

/// RAII guard for an isolated temporary cache directory.
/// Automatically cleans up the directory when dropped.
#[derive(Debug)]
pub struct CacheGuard {
    path: PathBuf,
}

impl CacheGuard {
    /// Returns the filesystem path to the isolated cache directory.
    #[must_use]
    #[allow(dead_code)]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the filesystem path as a string slice.
    ///
    /// # Panics
    ///
    /// Panics if the path is not valid UTF-8.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.path
            .to_str()
            .expect("valid utf-8 cache directory path")
    }
}

impl std::ops::Deref for CacheGuard {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

impl AsRef<Path> for CacheGuard {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl Drop for CacheGuard {
    fn drop(&mut self) {
        if self.path.exists() {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

/// Context holding runtime information for real Apollo integration tests.
#[derive(Clone, Debug)]
pub struct TestContext {
    pub config_url: String,
    #[allow(dead_code)]
    pub admin_url: String,
    pub run_dir: PathBuf,
    pub run_id: String,
    pub suite: String,
}

impl TestContext {
    /// Constructs a `TestContext` by reading required environment variables.
    ///
    /// # Panics
    ///
    /// Panics immediately if any required environment variable is missing,
    /// ensuring missing environment configuration never quietly passes.
    pub fn from_env() -> Self {
        let config_url = std::env::var("APOLLO_TEST_CONFIG_URL")
            .expect("APOLLO_TEST_CONFIG_URL must be set (run via scripts/apollo-test.sh)");
        let admin_url = std::env::var("APOLLO_TEST_ADMIN_URL")
            .expect("APOLLO_TEST_ADMIN_URL must be set (run via scripts/apollo-test.sh)");
        let run_dir = std::env::var("APOLLO_TEST_RUN_DIR")
            .expect("APOLLO_TEST_RUN_DIR must be set (run via scripts/apollo-test.sh)");
        let run_id = std::env::var("APOLLO_TEST_RUN_ID")
            .expect("APOLLO_TEST_RUN_ID must be set (run via scripts/apollo-test.sh)");
        let suite = std::env::var("APOLLO_TEST_SUITE")
            .expect("APOLLO_TEST_SUITE must be set (run via scripts/apollo-test.sh)");

        Self {
            config_url,
            admin_url,
            run_dir: PathBuf::from(run_dir),
            run_id,
            suite,
        }
    }

    /// Creates an isolated temporary cache directory under the run directory.
    ///
    /// # Panics
    ///
    /// Panics if the directory cannot be created.
    pub fn new_cache_dir(&self, label: &str) -> CacheGuard {
        let id = CACHE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = self
            .run_dir
            .join("cache")
            .join(&self.suite)
            .join(format!("{label}-{id}"));
        if dir.exists() {
            let _ = std::fs::remove_dir_all(&dir);
        }
        std::fs::create_dir_all(&dir).expect("failed to create isolated cache directory");
        CacheGuard { path: dir }
    }

    /// Returns a pre-configured `ClientConfigBuilder` pointing to the real Apollo `ConfigService`,
    /// paired with an isolated `CacheGuard` managing the lifetime of this client's cache directory.
    ///
    /// # Panics
    ///
    /// Panics if the cache directory path contains invalid UTF-8.
    pub fn client_builder(
        &self,
        app_id: &str,
        cache_label: &str,
    ) -> (ClientConfigBuilder, CacheGuard) {
        let guard = self.new_cache_dir(cache_label);
        let builder = ClientConfig::builder(app_id, &self.config_url).cache_dir(guard.as_str());
        (builder, guard)
    }
}
