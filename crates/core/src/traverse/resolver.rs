use std::path::PathBuf;

use oxc_resolver::Resolution;

use crate::resolve_file_path;

pub struct ProjectResolver {
    tsconfig_path: Option<PathBuf>,
}

impl ProjectResolver {
    pub fn new(tsconfig_path: Option<PathBuf>) -> Self {
        Self { tsconfig_path }
    }

    pub fn resolve(&self, dir: &PathBuf, specifier: &str) -> Result<Resolution, String> {
        resolve_file_path(dir, specifier, self.tsconfig_path.as_ref())
    }
}
