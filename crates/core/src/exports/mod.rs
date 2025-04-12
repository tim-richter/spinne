use std::path::PathBuf;
use spinne_logger::Logger;

/// Represents the entry points to analyze for exports
#[derive(Debug)]
pub struct Exports {
    /// The entry points to analyze
    pub entry_points: Vec<PathBuf>,
}

impl Exports {
    /// Creates a new Exports instance with the specified entry points
    pub fn new(entry_points: Vec<PathBuf>) -> Self {
        Self { entry_points }
    }

    /// Analyzes the entry points to find exported items
    pub fn analyze(&self) {
        for entry_point in &self.entry_points {
            if !entry_point.exists() {
                Logger::warn(&format!("Entry point does not exist: {}", entry_point.display()));
                continue;
            }

            Logger::info(&format!("Analyzing entry point: {}", entry_point.display()));
            // TODO: Use the traverse module to analyze the entry point
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn create_test_file(temp_dir: &tempfile::TempDir, path: &str) -> PathBuf {
        let full_path = temp_dir.path().join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&full_path, "").unwrap();
        full_path
    }

    #[test]
    fn test_new_exports() {
        let temp_dir = tempdir().unwrap();
        let entry_point = create_test_file(&temp_dir, "src/index.ts");
        
        let exports = Exports::new(vec![entry_point]);
        assert_eq!(exports.entry_points.len(), 1);
    }

    #[test]
    fn test_analyze_nonexistent_entry_point() {
        let temp_dir = tempdir().unwrap();
        let nonexistent_path = temp_dir.path().join("src/nonexistent.ts");
        
        let exports = Exports::new(vec![nonexistent_path]);
        exports.analyze(); // Should log a warning but not panic
    }
} 