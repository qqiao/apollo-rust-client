use json::Json;
use properties::Properties;

pub mod json;
pub mod properties;

#[derive(Clone, Debug)]
pub enum Namespace {
    Properties(Properties),
    Json(Json),
    // Yaml(T),
    // Xml(T),
    Text(String),
}
enum NamespaceType {
    Properties,
    Json,
    Yaml,
    Xml,
    Text,
}

fn get_namespace_type(namespace: &str) -> NamespaceType {
    let parts = namespace.split(".").collect::<Vec<&str>>();
    if parts.len() == 1 {
        NamespaceType::Properties
    } else {
        match parts.last().unwrap().to_lowercase().as_str() {
            "json" => NamespaceType::Json,
            "yaml" | "yml" => NamespaceType::Yaml,
            "xml" => NamespaceType::Xml,
            "txt" => NamespaceType::Text,
            _ => todo!(),
        }
    }
}

pub(crate) fn get_namespace(namespace: &str, value: serde_json::Value) -> Namespace {
    match get_namespace_type(namespace) {
        NamespaceType::Properties => Namespace::Properties(properties::Properties::from(value)),
        NamespaceType::Json => Namespace::Json(json::Json::from(value)),
        _ => todo!(),
    }
}
