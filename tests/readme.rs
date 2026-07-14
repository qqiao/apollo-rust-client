//! Compile checks for the public Rust API shown in `README.md`.

#![cfg(not(target_arch = "wasm32"))]

use apollo_rust_client::{Client, client_config::ClientConfig, namespace::Namespace};

#[allow(dead_code)]
async fn readme_quick_start_compiles() -> Result<(), Box<dyn std::error::Error>> {
    let config = ClientConfig::builder("your_app_id", "http://apollo-server:8080")
        .secret("your_apollo_secret")
        .label("production")
        .ip("192.168.1.100")
        .cache_ttl(600)
        .refresh_interval(30)
        .request_timeout(10)
        .build()?;
    let mut client = Client::new(config)?;
    client.start().await?;

    match client.namespace("application").await? {
        Namespace::Properties(properties) => {
            let _ = properties.get_string("app.name");
        }
        Namespace::Json(json) => {
            let _: serde_json::Value = json.to_object()?;
        }
        Namespace::Yaml(yaml) => {
            let _: serde_json::Value = yaml.to_object()?;
        }
        Namespace::Text(text) => drop(text),
    }

    client
        .add_listener("application", std::sync::Arc::new(|_| {}))
        .await;
    client.refresh("application").await?;
    client.stop().await;
    Ok(())
}
