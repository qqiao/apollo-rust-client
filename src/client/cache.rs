use super::Error;
use serde::de::DeserializeOwned;

pub struct Cache {}

impl Cache {
    pub fn get_config<D: DeserializeOwned>(&self, key: &str) -> Result<D, Error> {
        todo!()
    }
}
