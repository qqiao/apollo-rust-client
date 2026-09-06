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

    /// Returns the reserved mutable namespace for this test suite.
    pub fn update_namespace(&self) -> &'static str {
        match self.suite.as_str() {
            "native" => "updates-native",
            "rustls" => "updates-rustls",
            "wasm" => "updates-wasm",
            _ => "updates-native",
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

    /// Saves an item in Apollo via AdminService without publishing.
    ///
    /// # Panics
    ///
    /// Panics if the underlying node script fails or returns nonzero.
    pub async fn set_item(&self, app_id: &str, namespace: &str, key: &str, value: &str) {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let script = repo_root.join("scripts").join("apollo-fixtures.mjs");
        let fixtures = repo_root.join("tests").join("apollo").join("fixtures.json");
        let admin_url = self.admin_url.clone();
        let app_id = app_id.to_string();
        let namespace = namespace.to_string();
        let key = key.to_string();
        let value = value.to_string();

        tokio::task::spawn_blocking(move || {
            let output = std::process::Command::new("node")
                .arg(&script)
                .arg("set-item")
                .arg("--admin-url")
                .arg(&admin_url)
                .arg("--app")
                .arg(&app_id)
                .arg("--namespace")
                .arg(&namespace)
                .arg("--key")
                .arg(&key)
                .arg("--value")
                .arg(&value)
                .arg("--fixtures")
                .arg(&fixtures)
                .output()
                .expect("failed to execute node set-item");

            if !output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                panic!(
                    "set-item failed (exit {:?}):\nstdout: {}\nstderr: {}",
                    output.status.code(),
                    stdout,
                    stderr
                );
            }
        })
        .await
        .expect("spawn_blocking panicked");
    }

    /// Publishes a release for a mutable namespace in Apollo via AdminService.
    ///
    /// # Panics
    ///
    /// Panics if the underlying node script fails or returns nonzero.
    pub async fn publish(&self, app_id: &str, namespace: &str, release_name: Option<&str>) {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let script = repo_root.join("scripts").join("apollo-fixtures.mjs");
        let fixtures = repo_root.join("tests").join("apollo").join("fixtures.json");
        let admin_url = self.admin_url.clone();
        let app_id = app_id.to_string();
        let namespace = namespace.to_string();
        let release_name = release_name.map(str::to_string);

        tokio::task::spawn_blocking(move || {
            let mut cmd = std::process::Command::new("node");
            cmd.arg(&script)
                .arg("publish")
                .arg("--admin-url")
                .arg(&admin_url)
                .arg("--app")
                .arg(&app_id)
                .arg("--namespace")
                .arg(&namespace)
                .arg("--fixtures")
                .arg(&fixtures);
            if let Some(ref name) = release_name {
                cmd.arg("--name").arg(name);
            }
            let output = cmd.output().expect("failed to execute node publish");

            if !output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                panic!(
                    "publish failed (exit {:?}):\nstdout: {}\nstderr: {}",
                    output.status.code(),
                    stdout,
                    stderr
                );
            }
        })
        .await
        .expect("spawn_blocking panicked");
    }

    /// Sets an item and publishes a release for a mutable namespace in one operation.
    ///
    /// # Panics
    ///
    /// Panics if the underlying node script fails or returns nonzero.
    pub async fn set_and_publish(&self, app_id: &str, namespace: &str, key: &str, value: &str) {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let script = repo_root.join("scripts").join("apollo-fixtures.mjs");
        let fixtures = repo_root.join("tests").join("apollo").join("fixtures.json");
        let admin_url = self.admin_url.clone();
        let app_id = app_id.to_string();
        let namespace = namespace.to_string();
        let key = key.to_string();
        let value = value.to_string();

        tokio::task::spawn_blocking(move || {
            let output = std::process::Command::new("node")
                .arg(&script)
                .arg("set-and-publish")
                .arg("--admin-url")
                .arg(&admin_url)
                .arg("--app")
                .arg(&app_id)
                .arg("--namespace")
                .arg(&namespace)
                .arg("--key")
                .arg(&key)
                .arg("--value")
                .arg(&value)
                .arg("--fixtures")
                .arg(&fixtures)
                .output()
                .expect("failed to execute node set-and-publish");

            if !output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                panic!(
                    "set-and-publish failed (exit {:?}):\nstdout: {}\nstderr: {}",
                    output.status.code(),
                    stdout,
                    stderr
                );
            }
        })
        .await
        .expect("spawn_blocking panicked");
    }

    /// Polls ConfigService directly until the expected key-value pair is returned,
    /// or panics when the timeout is reached.
    ///
    /// # Panics
    ///
    /// Panics if the expected value is not visible within `timeout`.
    pub async fn wait_for_config_service_value(
        &self,
        app_id: &str,
        cluster: &str,
        namespace: &str,
        expected_key: &str,
        expected_val: &str,
        timeout: std::time::Duration,
    ) {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(3))
            .build()
            .expect("reqwest client build failed");
        let url = format!(
            "{}/configfiles/json/{}/{}/{}",
            self.config_url.trim_end_matches('/'),
            app_id,
            cluster,
            namespace
        );

        let start = std::time::Instant::now();
        while start.elapsed() < timeout {
            let matched = async {
                let resp = client.get(&url).send().await.ok()?;
                if !resp.status().is_success() {
                    return None;
                }
                let json = resp.json::<serde_json::Value>().await.ok()?;
                let val = json.get(expected_key)?.as_str()?;
                Some(val == expected_val)
            };
            if matched.await == Some(true) {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }

        panic!(
            "Timed out waiting for ConfigService at {url} to return {expected_key}={expected_val:?} within {timeout:?}"
        );
    }
}
