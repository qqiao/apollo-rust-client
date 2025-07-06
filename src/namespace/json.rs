use serde::de::DeserializeOwned;

#[derive(Clone, Debug)]
pub struct Json {
    value: serde_json::Value,
}

impl Json {
    pub fn to_object<T: DeserializeOwned>(&self) -> T {
        todo!()
    }
}

impl From<serde_json::Value> for Json {
    fn from(value: serde_json::Value) -> Self {
        Self { value }
    }
}
