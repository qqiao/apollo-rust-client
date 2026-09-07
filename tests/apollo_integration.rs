//! Real Apollo integration test suite for native Rust targets.
//!
//! Tests the public API against a genuine running Apollo `ConfigService` instance.
//! These tests require real Apollo services managed by `scripts/apollo-test.sh`.

#![cfg(not(target_arch = "wasm32"))]
#![allow(clippy::pedantic)]

#[path = "apollo/support/mod.rs"]
mod support;

use apollo_rust_client::{Client, client_config::ClientConfig, namespace::Namespace};
use std::sync::{Arc, Mutex};
use support::TestContext;

#[tokio::test]
#[ignore = "requires scripts/test.sh integration"]
async fn real_apollo_formats_and_identity() {
    let ctx = TestContext::from_env();

    // App 101010101, default cluster: Properties namespace "application"
    let (config_builder, _guard) = ctx.client_builder("101010101", "formats-plain-default");
    let client = Client::new(config_builder.build().expect("valid config"))
        .expect("failed to create client");

    let ns = client
        .namespace("application")
        .await
        .expect("failed to get application namespace");
    match ns {
        Namespace::Properties(props) => {
            assert_eq!(
                props.get_string("stringValue"),
                Some("string value".to_string()),
                "stringValue mismatch"
            );
            assert_eq!(props.get_int("intValue"), Some(42), "intValue mismatch");
            assert!(
                (props.get_float("floatValue").expect("floatValue") - 4.20).abs() < 1e-6,
                "floatValue mismatch"
            );
            assert_eq!(
                props.get_bool("boolValue"),
                Some(false),
                "boolValue mismatch"
            );
            assert_eq!(
                props.get_bool("grayScaleValue"),
                Some(false),
                "grayScaleValue baseline should be false"
            );
            assert_eq!(
                props.get_string("identity"),
                Some("plain-default".to_string()),
                "identity mismatch"
            );
            assert_eq!(
                props.get_string("fixtureRunId"),
                Some(ctx.run_id.clone()),
                "fixtureRunId must match this test run ID"
            );
            assert_eq!(
                props.get_string("missingValue"),
                None,
                "missingValue should not exist"
            );
        }
        other => panic!("expected Namespace::Properties, got {other:?}"),
    }

    // Nonexistent private namespace should produce a server retrieval error (HTTP 404)
    let err = client
        .namespace("nonexistent-private-namespace")
        .await
        .expect_err("nonexistent namespace should fail");
    let err_str = err.to_string();
    assert!(
        err_str.contains("HTTP 404"),
        "expected HTTP 404 in error message, got: {err_str}"
    );

    // JSON format: "application.json"
    let json_ns = client
        .namespace("application.json")
        .await
        .expect("failed to get application.json namespace");
    match json_ns {
        Namespace::Json(json) => {
            let obj: serde_json::Value = json.to_object().expect("deserialization to json value");
            assert_eq!(obj["host"], "localhost");
            assert_eq!(obj["port"], 8080);
            assert_eq!(obj["run"], true);
        }
        other => panic!("expected Namespace::Json, got {other:?}"),
    }

    // YAML format: "application.yml"
    let yml_ns = client
        .namespace("application.yml")
        .await
        .expect("failed to get application.yml namespace");
    match yml_ns {
        Namespace::Yaml(yaml) => {
            let val: noyalib::Value = yaml.to_object().expect("deserialization to yaml value");
            assert_eq!(val["host"].as_str(), Some("localhost"));
            assert_eq!(val["port"].as_i64(), Some(8080));
            assert_eq!(val["run"].as_bool(), Some(true));
        }
        other => panic!("expected Namespace::Yaml, got {other:?}"),
    }

    // YAML format: "application.yaml"
    let yaml_ns = client
        .namespace("application.yaml")
        .await
        .expect("failed to get application.yaml namespace");
    match yaml_ns {
        Namespace::Yaml(yaml) => {
            let val: noyalib::Value = yaml.to_object().expect("deserialization to yaml value");
            assert_eq!(val["host"].as_str(), Some("localhost"));
            assert_eq!(val["port"].as_i64(), Some(8080));
            assert_eq!(val["run"].as_bool(), Some(true));
        }
        other => panic!("expected Namespace::Yaml, got {other:?}"),
    }

    // Text format: "readme.txt"
    let txt_ns = client
        .namespace("readme.txt")
        .await
        .expect("failed to get readme.txt namespace");
    match txt_ns {
        Namespace::Text(content) => {
            assert_eq!(content, "plain text configuration\nsecond line\n");
        }
        other => panic!("expected Namespace::Text, got {other:?}"),
    }

    // Properties format: "config"
    let config_ns = client
        .namespace("config")
        .await
        .expect("failed to get config namespace");
    match config_ns {
        Namespace::Properties(props) => {
            assert_eq!(
                props.get_string("publicValue"),
                Some("properties".to_string())
            );
        }
        other => panic!("expected Namespace::Properties, got {other:?}"),
    }

    // Wire normalization: "config.properties" alias for "config"
    let config_prop_ns = client
        .namespace("config.properties")
        .await
        .expect("failed to get config.properties namespace");
    match config_prop_ns {
        Namespace::Properties(props) => {
            assert_eq!(
                props.get_string("publicValue"),
                Some("properties".to_string()),
                "config.properties alias mismatch"
            );
        }
        other => panic!("expected Namespace::Properties, got {other:?}"),
    }

    // Associated public namespace: "FX.apollo"
    let assoc_ns = client
        .namespace("FX.apollo")
        .await
        .expect("failed to get associated FX.apollo namespace");
    match assoc_ns {
        Namespace::Properties(props) => {
            assert_eq!(
                props.get_string("publicValue"),
                Some("associated".to_string())
            );
            assert_eq!(
                props.get_string("identity"),
                Some("public-owner".to_string())
            );
        }
        other => panic!("expected Namespace::Properties, got {other:?}"),
    }

    // Cluster selection: "integration" cluster on 101010101
    let (cluster_builder, _cluster_guard) =
        ctx.client_builder("101010101", "formats-integration-cluster");
    let cluster_client = Client::new(
        cluster_builder
            .cluster("integration")
            .build()
            .expect("valid config"),
    )
    .expect("failed to create client");
    let cluster_ns = cluster_client
        .namespace("application")
        .await
        .expect("failed to get application namespace in integration cluster");
    match cluster_ns {
        Namespace::Properties(props) => {
            assert_eq!(
                props.get_string("stringValue"),
                Some("cluster value".to_string())
            );
            assert_eq!(
                props.get_string("identity"),
                Some("plain-integration".to_string())
            );
            assert_eq!(
                props.get_string("fixtureRunId"),
                Some(ctx.run_id.clone()),
                "cluster fixtureRunId must match test run ID"
            );
        }
        other => panic!("expected Namespace::Properties, got {other:?}"),
    }

    // Second distinct application identity: "101010103" (public owner app reading FX.apollo)
    let (owner_builder, _owner_guard) = ctx.client_builder("101010103", "formats-public-owner");
    let owner_client =
        Client::new(owner_builder.build().expect("valid config")).expect("failed to create client");
    let owner_ns = owner_client
        .namespace("FX.apollo")
        .await
        .expect("failed to get FX.apollo namespace on owner app");
    match owner_ns {
        Namespace::Properties(props) => {
            assert_eq!(
                props.get_string("identity"),
                Some("public-owner".to_string()),
                "owner app identity mismatch"
            );
            assert_eq!(
                props.get_string("publicValue"),
                Some("associated".to_string()),
                "owner app publicValue mismatch"
            );
            assert_eq!(
                props.get_string("fixtureRunId"),
                Some(ctx.run_id.clone()),
                "owner app fixtureRunId must match test run ID"
            );
        }
        other => panic!("expected Namespace::Properties, got {other:?}"),
    }
}

#[tokio::test]
#[ignore = "requires scripts/test.sh integration"]
async fn real_apollo_access_key() {
    let ctx = TestContext::from_env();

    // App 101010102 has enforced access-key authentication (FILTER mode 0).
    // Valid secret: "apollo-integration-only-secret-v1"
    let (valid_builder, _valid_guard) = ctx.client_builder("101010102", "auth-valid-secret");
    let valid_client = Client::new(
        valid_builder
            .secret("apollo-integration-only-secret-v1")
            .build()
            .expect("valid config"),
    )
    .expect("failed to create client");
    let valid_ns = valid_client
        .namespace("application")
        .await
        .expect("authorized read with correct secret must succeed");
    match valid_ns {
        Namespace::Properties(props) => {
            assert_eq!(
                props.get_string("identity"),
                Some("secret-default".to_string())
            );
            assert_eq!(
                props.get_string("stringValue"),
                Some("string value".to_string())
            );
            assert_eq!(
                props.get_string("fixtureRunId"),
                Some(ctx.run_id.clone()),
                "fixtureRunId must match test run ID"
            );
        }
        other => panic!("expected Namespace::Properties, got {other:?}"),
    }

    // Missing secret with fresh isolated cache: must fail with unauthorized (HTTP 401)
    let (no_secret_builder, _no_secret_guard) =
        ctx.client_builder("101010102", "auth-missing-secret");
    let no_secret_client = Client::new(no_secret_builder.build().expect("valid config"))
        .expect("failed to create client");
    let err = no_secret_client
        .namespace("application")
        .await
        .expect_err("read without secret must fail when access-key is enforced");
    let err_str = err.to_string();
    assert!(
        err_str.contains("HTTP 401"),
        "expected HTTP 401 unauthorized in error message, got: {err_str}"
    );

    // Wrong secret with fresh isolated cache: must fail with unauthorized (HTTP 401)
    let (wrong_secret_builder, _wrong_secret_guard) =
        ctx.client_builder("101010102", "auth-wrong-secret");
    let wrong_secret_client = Client::new(
        wrong_secret_builder
            .secret("wrong-secret-value-12345")
            .build()
            .expect("valid config"),
    )
    .expect("failed to create client");
    let err = wrong_secret_client
        .namespace("application")
        .await
        .expect_err("read with wrong secret must fail");
    let err_str = err.to_string();
    assert!(
        err_str.contains("HTTP 401"),
        "expected HTTP 401 unauthorized in error message, got: {err_str}"
    );

    // Signed request with targeting parameters (IP and label) also succeeds when authorized
    let (targeted_builder, _targeted_guard) =
        ctx.client_builder("101010102", "auth-signed-targeting");
    let targeted_client = Client::new(
        targeted_builder
            .secret("apollo-integration-only-secret-v1")
            .ip("1.2.3.4")
            .label("GrayScale")
            .build()
            .expect("valid config"),
    )
    .expect("failed to create client");
    let targeted_ns = targeted_client
        .namespace("application")
        .await
        .expect("authorized signed request with targeting must succeed");
    match targeted_ns {
        Namespace::Properties(props) => {
            assert_eq!(
                props.get_string("identity"),
                Some("secret-default".to_string())
            );
            assert_eq!(
                props.get_bool("grayScaleValue"),
                Some(true),
                "matching targeting on protected app should receive grayscale value"
            );
            assert_eq!(
                props.get_string("fixtureRunId"),
                Some(ctx.run_id.clone()),
                "fixtureRunId must match test run ID"
            );
        }
        other => panic!("expected Namespace::Properties, got {other:?}"),
    }
}

#[tokio::test]
#[ignore = "requires scripts/test.sh integration"]
async fn real_apollo_grayscale() {
    let ctx = TestContext::from_env();

    // App 101010101 baseline grayScaleValue is "false".
    // Grayscale rule targets IP "1.2.3.4" or Label "GrayScale", branch value is "true".

    // Case 1: Matching IP ("1.2.3.4"), no label
    let (ip_match_builder, _guard1) = ctx.client_builder("101010101", "gray-ip-match");
    let ip_match_client = Client::new(
        ip_match_builder
            .ip("1.2.3.4")
            .build()
            .expect("valid config"),
    )
    .expect("failed to create client");
    let ns = ip_match_client
        .namespace("application")
        .await
        .expect("failed to get application namespace");
    match ns {
        Namespace::Properties(props) => {
            assert_eq!(
                props.get_bool("grayScaleValue"),
                Some(true),
                "matching IP should receive gray value"
            );
            assert_eq!(
                props.get_string("fixtureRunId"),
                Some(ctx.run_id.clone()),
                "fixtureRunId must match test run ID"
            );
        }
        other => panic!("expected Namespace::Properties, got {other:?}"),
    }

    // Case 2: Matching label ("GrayScale"), no IP
    let (label_match_builder, _guard2) = ctx.client_builder("101010101", "gray-label-match");
    let label_match_client = Client::new(
        label_match_builder
            .label("GrayScale")
            .build()
            .expect("valid config"),
    )
    .expect("failed to create client");
    let ns = label_match_client
        .namespace("application")
        .await
        .expect("failed to get application namespace");
    match ns {
        Namespace::Properties(props) => {
            assert_eq!(
                props.get_bool("grayScaleValue"),
                Some(true),
                "matching label should receive gray value"
            );
            assert_eq!(
                props.get_string("fixtureRunId"),
                Some(ctx.run_id.clone()),
                "fixtureRunId must match test run ID"
            );
        }
        other => panic!("expected Namespace::Properties, got {other:?}"),
    }

    // Case 3: Non-matching IP ("1.2.3.5")
    let (ip_nonmatch_builder, _guard3) = ctx.client_builder("101010101", "gray-ip-nonmatch");
    let ip_nonmatch_client = Client::new(
        ip_nonmatch_builder
            .ip("1.2.3.5")
            .build()
            .expect("valid config"),
    )
    .expect("failed to create client");
    let ns = ip_nonmatch_client
        .namespace("application")
        .await
        .expect("failed to get application namespace");
    match ns {
        Namespace::Properties(props) => {
            assert_eq!(
                props.get_bool("grayScaleValue"),
                Some(false),
                "non-matching IP should receive baseline value"
            );
            assert_eq!(
                props.get_string("fixtureRunId"),
                Some(ctx.run_id.clone()),
                "fixtureRunId must match test run ID"
            );
        }
        other => panic!("expected Namespace::Properties, got {other:?}"),
    }

    // Case 4: Non-matching label ("OtherLabel")
    let (label_nonmatch_builder, _guard4) = ctx.client_builder("101010101", "gray-label-nonmatch");
    let label_nonmatch_client = Client::new(
        label_nonmatch_builder
            .label("OtherLabel")
            .build()
            .expect("valid config"),
    )
    .expect("failed to create client");
    let ns = label_nonmatch_client
        .namespace("application")
        .await
        .expect("failed to get application namespace");
    match ns {
        Namespace::Properties(props) => {
            assert_eq!(
                props.get_bool("grayScaleValue"),
                Some(false),
                "non-matching label should receive baseline value"
            );
            assert_eq!(
                props.get_string("fixtureRunId"),
                Some(ctx.run_id.clone()),
                "fixtureRunId must match test run ID"
            );
        }
        other => panic!("expected Namespace::Properties, got {other:?}"),
    }

    // Case 5: Neither IP nor label specified
    let (neither_builder, _guard5) = ctx.client_builder("101010101", "gray-neither");
    let neither_client = Client::new(neither_builder.build().expect("valid config"))
        .expect("failed to create client");
    let ns = neither_client
        .namespace("application")
        .await
        .expect("failed to get application namespace");
    match ns {
        Namespace::Properties(props) => {
            assert_eq!(
                props.get_bool("grayScaleValue"),
                Some(false),
                "neither IP nor label should receive baseline value"
            );
            assert_eq!(
                props.get_string("fixtureRunId"),
                Some(ctx.run_id.clone()),
                "fixtureRunId must match test run ID"
            );
        }
        other => panic!("expected Namespace::Properties, got {other:?}"),
    }
}

#[tokio::test]
#[ignore = "requires scripts/test.sh integration"]
async fn real_apollo_release_refresh_and_listener() {
    let ctx = TestContext::from_env();
    let app_id = "101010101";
    let update_ns = ctx.update_namespace();

    // 1. Scoped precondition: Restore reserved update namespace to baseline
    ctx.set_and_publish(app_id, update_ns, "value", "baseline")
        .await;
    ctx.wait_for_config_service_value(
        app_id,
        "default",
        update_ns,
        "value",
        "baseline",
        std::time::Duration::from_secs(60),
    )
    .await;

    // Load with fresh client and isolated cache
    let (builder, _guard) = ctx.client_builder(app_id, "release-listener");
    let client =
        Client::new(builder.build().expect("valid config")).expect("failed to create client");

    // Capture listener events separately from changes
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_cb = events.clone();
    let listener = Arc::new(move |res: Result<Namespace, apollo_rust_client::Error>| {
        let Some(val) = res.ok().and_then(|ns| match ns {
            Namespace::Properties(props) => props.get_string("value"),
            _ => None,
        }) else {
            return;
        };
        events_cb.lock().unwrap().push(val);
    });

    client.add_listener(update_ns, listener).await;

    // Initial namespace load: listener must receive initial event
    let initial_ns = client
        .namespace(update_ns)
        .await
        .expect("initial namespace load");
    match initial_ns {
        Namespace::Properties(props) => {
            assert_eq!(props.get_string("value"), Some("baseline".to_string()));
        }
        other => panic!("expected Properties, got {other:?}"),
    }
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert_eq!(
        events.lock().unwrap().as_slice(),
        &["baseline".to_string()],
        "initial load must trigger initial listener event"
    );

    // 2. Save value="edited-unpublished" with set-item (without publishing)
    ctx.set_item(app_id, update_ns, "value", "edited-unpublished")
        .await;

    // Direct independent ConfigService request must still see baseline
    let probe_client = reqwest::Client::new();
    let probe_url = format!(
        "{}/configfiles/json/{app_id}/default/{update_ns}",
        ctx.config_url.trim_end_matches('/')
    );
    let probe_resp = probe_client
        .get(&probe_url)
        .send()
        .await
        .expect("probe request");
    let probe_json: serde_json::Value = probe_resp.json().await.expect("probe json");
    assert_eq!(
        probe_json.get("value").and_then(|v| v.as_str()),
        Some("baseline"),
        "unpublished edit must NOT be visible to ConfigService"
    );

    // Explicit client refresh: still sees baseline and produces NO changed-value listener event
    client.refresh(update_ns).await.expect("refresh succeeds");
    let refreshed_ns = client.namespace(update_ns).await.expect("namespace read");
    match refreshed_ns {
        Namespace::Properties(props) => {
            assert_eq!(props.get_string("value"), Some("baseline".to_string()));
        }
        other => panic!("expected Properties, got {other:?}"),
    }
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert_eq!(
        events.lock().unwrap().as_slice(),
        &["baseline".to_string()],
        "refresh on unchanged server data must not trigger new listener event"
    );

    // 3. Publish that item
    ctx.publish(app_id, update_ns, None).await;

    // Independently wait until ConfigService exposes "edited-unpublished"
    ctx.wait_for_config_service_value(
        app_id,
        "default",
        update_ns,
        "value",
        "edited-unpublished",
        std::time::Duration::from_secs(60),
    )
    .await;

    // Explicitly refresh the client and require the corresponding event and value
    client
        .refresh(update_ns)
        .await
        .expect("refresh after publish succeeds");
    let updated_ns = client.namespace(update_ns).await.expect("namespace read");
    match updated_ns {
        Namespace::Properties(props) => {
            assert_eq!(
                props.get_string("value"),
                Some("edited-unpublished".to_string())
            );
        }
        other => panic!("expected Properties, got {other:?}"),
    }
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert_eq!(
        events.lock().unwrap().as_slice(),
        &["baseline".to_string(), "edited-unpublished".to_string()],
        "refresh after publish must trigger changed listener event"
    );

    // Refresh unchanged data once more and assert NO extra event from that operation
    client
        .refresh(update_ns)
        .await
        .expect("second refresh succeeds");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert_eq!(
        events.lock().unwrap().as_slice(),
        &["baseline".to_string(), "edited-unpublished".to_string()],
        "subsequent refresh of identical data must not trigger extra listener event"
    );
}

#[tokio::test]
#[ignore = "requires scripts/test.sh integration"]
async fn real_apollo_polling() {
    let ctx = TestContext::from_env();
    let app_id = "101010101";
    let update_ns = ctx.update_namespace();

    // 1. Scoped precondition: Restore reserved update namespace to baseline
    ctx.set_and_publish(app_id, update_ns, "value", "baseline")
        .await;
    ctx.wait_for_config_service_value(
        app_id,
        "default",
        update_ns,
        "value",
        "baseline",
        std::time::Duration::from_secs(60),
    )
    .await;

    // Create client with 1-second refresh interval and isolated cache
    let (builder, _guard) = ctx.client_builder(app_id, "polling");
    let mut client = Client::new(builder.refresh_interval(1).build().expect("valid config"))
        .expect("failed to create client");

    let events = Arc::new(Mutex::new(Vec::new()));
    let events_cb = events.clone();
    let listener = Arc::new(move |res: Result<Namespace, apollo_rust_client::Error>| {
        let Some(val) = res.ok().and_then(|ns| match ns {
            Namespace::Properties(props) => props.get_string("value"),
            _ => None,
        }) else {
            return;
        };
        events_cb.lock().unwrap().push(val);
    });

    client.add_listener(update_ns, listener).await;

    // Load initial namespace
    let initial_ns = client.namespace(update_ns).await.expect("initial load");
    match initial_ns {
        Namespace::Properties(props) => {
            assert_eq!(props.get_string("value"), Some("baseline".to_string()));
        }
        other => panic!("expected Properties, got {other:?}"),
    }

    // Start background polling
    client.start().await.expect("failed to start polling");

    // Publish new value to Apollo
    ctx.set_and_publish(app_id, update_ns, "value", "polling-published")
        .await;
    ctx.wait_for_config_service_value(
        app_id,
        "default",
        update_ns,
        "value",
        "polling-published",
        std::time::Duration::from_secs(60),
    )
    .await;

    // Bounded observation: wait for polling background loop to update client without manual refresh
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(60);
    let mut observed = false;

    while start.elapsed() < timeout {
        let is_updated = matches!(
            client.namespace(update_ns).await,
            Ok(Namespace::Properties(ref props)) if props.get_string("value").as_deref() == Some("polling-published")
        );
        if is_updated {
            observed = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    // Ensure client background polling is stopped promptly on success or failure
    client.stop().await;

    assert!(
        observed,
        "polling client failed to observe new published value within {timeout:?}"
    );

    // Verify listener captured the change event
    let captured = events.lock().unwrap().clone();
    assert!(
        captured.contains(&"polling-published".to_string()),
        "listener events must contain polling-published: {captured:?}"
    );
}

#[tokio::test]
#[ignore = "requires scripts/test.sh integration"]
async fn real_apollo_preload_and_persistence() {
    let ctx = TestContext::from_env();
    let app_id = "101010101";

    // 1. Duplicate-preload assertions
    let (builder1, guard1) = ctx.client_builder(app_id, "preload-duplicates");
    let client1 = Client::new(builder1.build().expect("valid config")).expect("create client1");

    // Preload duplicate namespaces and multiple formats simultaneously
    client1
        .preload(&["application", "application", "application.json", "config", "application"])
        .await
        .expect("preload with duplicate namespaces must succeed without error");

    // Verify preloaded values
    let app_ns = client1
        .namespace("application")
        .await
        .expect("get application");
    match app_ns {
        Namespace::Properties(props) => {
            assert_eq!(
                props.get_string("stringValue"),
                Some("string value".to_string())
            );
            assert_eq!(props.get_string("fixtureRunId"), Some(ctx.run_id.clone()));
        }
        other => panic!("expected Properties, got {other:?}"),
    }

    let json_ns = client1
        .namespace("application.json")
        .await
        .expect("get application.json");
    match json_ns {
        Namespace::Json(json) => {
            let obj: serde_json::Value = json.to_object().expect("deserialization to json value");
            assert_eq!(obj["host"], "localhost");
            assert_eq!(obj["port"], 8080);
            assert_eq!(obj["run"], true);
        }
        other => panic!("expected Json, got {other:?}"),
    }

    // 2. Native same-identity completed-persistence smoke
    // Confirm client1 wrote cache files into guard1's cache storage directory
    let cache_storage_dir = guard1.path().join("apollo-rust-client").join("config-cache");
    assert!(cache_storage_dir.exists(), "cache storage directory must exist: {:?}", cache_storage_dir);
    let cache_files: Vec<_> = std::fs::read_dir(&cache_storage_dir)
        .expect("read cache storage dir")
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().ends_with(".cache.json"))
        .collect();
    assert!(
        !cache_files.is_empty(),
        "cache storage directory must contain persisted cache artifacts (.cache.json)"
    );

    // Create a second matching client sharing the same cache directory
    let client2_config = ClientConfig::builder(app_id, &ctx.config_url)
        .cache_dir(guard1.as_str())
        .build()
        .expect("valid client2 config");
    let client2 = Client::new(client2_config).expect("create client2");

    let app_ns2 = client2
        .namespace("application")
        .await
        .expect("client2 get application from persistent cache");
    match app_ns2 {
        Namespace::Properties(props) => {
            assert_eq!(
                props.get_string("stringValue"),
                Some("string value".to_string())
            );
            assert_eq!(props.get_string("fixtureRunId"), Some(ctx.run_id.clone()));
        }
        other => panic!("expected Properties, got {other:?}"),
    }
}
