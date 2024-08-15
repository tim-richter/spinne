use spinne::ProjectTraverser; // Replace `my_crate` with your crate's name
use std::collections::HashSet;

#[test]
fn test_traverse_simple_project() {
    // Set up a temporary directory with a mock project structure
    let temp_dir = tempfile::tempdir().unwrap();
    let entry_file = temp_dir.path().join("entry_file.ts");

    // Write a mock TypeScript file (or any file) to the entry path
    std::fs::write(&entry_file, "import './some_module';").unwrap();

    // Initialize the ProjectTraverser
    let traverser = ProjectTraverser::new();

    // Call the traverse method and check the result
    let result = traverser.traverse(&entry_file);

    // Assert that the result is Ok
    assert!(result.is_ok());
}

#[test]
fn test_traverse_circular_dependencies() {
    // Set up a temporary directory with mock project files
    let temp_dir = tempfile::tempdir().unwrap();
    let file_a = temp_dir.path().join("file_a.ts");
    let file_b = temp_dir.path().join("file_b.ts");

    // Write files that reference each other
    std::fs::write(&file_a, "import './file_b';").unwrap();
    std::fs::write(&file_b, "import './file_a';").unwrap();

    // Initialize the ProjectTraverser
    let traverser = ProjectTraverser::new();

    // Call the traverse method and check the result
    let result = traverser.traverse(&file_a).expect("Traversal failed");

    let mut expected_files = HashSet::new();
    expected_files.insert(file_a.canonicalize().unwrap());
    expected_files.insert(file_b.canonicalize().unwrap());
    // Assert that the result is Ok and no infinite loop occurred
    assert_eq!(result, expected_files);
}
