#![cfg(not(target_arch = "wasm32"))]

use apollo_rust_client::{CacheError, Error};

#[test]
fn match_http_status_externally() {
    let cache_err = CacheError::HttpStatus {
        status: 429,
        body: "rate limited".into(),
    };
    let err: Error = cache_err.into();

    match err {
        Error::Cache(CacheError::HttpStatus { status, body }) => {
            assert_eq!(status, 429);
            assert_eq!(body, "rate limited");
        }
        other => panic!("expected Error::Cache(CacheError::HttpStatus), got: {other:?}"),
    }
}

#[test]
fn match_timeout_externally() {
    let cache_err = CacheError::Timeout { seconds: 10 };
    let err: Error = cache_err.into();

    match err {
        Error::Cache(CacheError::Timeout { seconds }) => {
            assert_eq!(seconds, 10);
        }
        other => panic!("expected Error::Cache(CacheError::Timeout), got: {other:?}"),
    }
}

#[test]
fn match_coalesced_refresh_externally() {
    let cache_err = CacheError::CoalescedRefresh("failure snapshot".into());
    let err: Error = cache_err.into();

    match err {
        Error::Cache(CacheError::CoalescedRefresh(ref snapshot)) => {
            assert_eq!(snapshot, "failure snapshot");
            let display = err.to_string();
            assert!(
                display.contains("failure snapshot"),
                "display should contain failure snapshot: {display}"
            );
        }
        other => panic!("expected Error::Cache(CacheError::CoalescedRefresh), got: {other:?}"),
    }
}

#[test]
fn preserve_existing_error_cache_matching_and_std_error() {
    let cache_err = CacheError::Timeout { seconds: 5 };
    let err: Error = cache_err.into();

    match err {
        Error::Cache(inner) => {
            let dyn_err: &dyn std::error::Error = &inner;
            assert!(dyn_err.to_string().contains('5'));
        }
        other => panic!("expected Error::Cache, got: {other:?}"),
    }
}
