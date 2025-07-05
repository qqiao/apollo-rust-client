use properties::Properties;

pub mod properties;

#[derive(Clone)]
pub enum Namespace<T> {
    Properties(Properties),
    Json(T),
    Yaml(T),
    Xml(T),
    Text(String),
}
