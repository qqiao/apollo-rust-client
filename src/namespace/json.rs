use serde::de::DeserializeOwned;

#[derive(Clone, Debug)]
pub struct Json {}

impl Json {
    pub fn to_object<T: DeserializeOwned>(&self) -> T {
        todo!()
    }
}
