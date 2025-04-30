use cache::Cache;
use config::Config;

pub mod cache;
pub mod config;

#[derive(Debug, thiserror::Error)]
pub enum Error {}

pub struct Client {}

impl Client {
    pub fn new(config: Config) -> Self {
        todo!()
    }

    pub fn get_cache(&self, name_space: &str) -> &Cache {
        todo!()
    }
}
