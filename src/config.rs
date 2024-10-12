use std::path::PathBuf;
use std::sync::Arc;

pub struct Config {
    pub base_url: PathBuf,
    pub paths: Vec<(String, Vec<String>)>,
}

impl Config {
    pub fn new(base_url: PathBuf, paths: Vec<(String, Vec<String>)>) -> Arc<Self> {
        Arc::new(Self { base_url, paths })
    }
}
