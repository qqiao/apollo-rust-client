use json::Json;
use properties::Properties;

pub mod json;
pub mod properties;

#[derive(Clone)]
pub enum Namespace {
    Properties(Properties),
    Json(Json),
    // Yaml(T),
    // Xml(T),
    Text(String),
}
