//! Real Apollo integration test suite for native Rust targets.
//!
//! Tests the public API against a genuine running Apollo `ConfigService` instance.
//! These tests require real Apollo services managed by `scripts/apollo-test.sh`.

#![cfg(not(target_arch = "wasm32"))]
#![allow(clippy::pedantic)]

#[path = "apollo/support/mod.rs"]
mod support;

use apollo_rust_client::{Client, namespace::Namespace};
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
