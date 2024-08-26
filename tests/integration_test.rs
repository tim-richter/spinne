use petgraph::prelude::DiGraphMap;
use petgraph::dot::{Dot, Config};
use spinne::ProjectTraverser;
use std::collections::HashSet;
use std::fs::File;
use std::io::Write;

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
    let (result, _bla) = traverser.traverse(&file_a).expect("Traversal failed");

    let mut expected_files = HashSet::new();
    expected_files.insert(file_a.canonicalize().unwrap());
    expected_files.insert(file_b.canonicalize().unwrap());
    // Assert that the result is Ok and no infinite loop occurred
    assert_eq!(result, expected_files);
}

#[test]
fn test_project_traverser_basic() {
    // Create a temporary directory to simulate a project
    let dir = tempfile::tempdir().unwrap();
    let dir_path = dir.path();

    // Create a simple React component structure
    let entry_path = dir_path.join("App.tsx");
    let child_path = dir_path.join("ChildComponent.tsx");

    // Write the entry file
    let mut entry_file = std::fs::File::create(&entry_path).unwrap();
    write!(
        entry_file,
        r#"
                import React from 'react';
                import ChildComponent from './ChildComponent';

                function App() {{
                    return <ChildComponent />;
                }}

                export default App;
            "#
    )
        .unwrap();

    // Write the child component file
    let mut child_file = File::create(&child_path).unwrap();
    write!(
        child_file,
        r#"
                import React from 'react';

                const ChildComponent = () => {{
                    return <div>Child Component</div>;
                }};

                export default ChildComponent;
            "#
    )
        .unwrap();

    // Now, let's traverse the project
    let traverser = ProjectTraverser::new();
    let (result, _bla) = traverser.traverse(&entry_path).unwrap();

    // Ensure all files were visited
    let mut expected_files = HashSet::new();
    expected_files.insert(entry_path.canonicalize().unwrap());
    expected_files.insert(child_path.canonicalize().unwrap());

    assert_eq!(result, expected_files);

    // Optionally, you can capture the output using a logging or buffer capture technique
    // and assert the correct graph was generated. This example doesn't capture the print output.
}

#[test]
fn test_project_traverser_with_nested_components() {
    // Create a temporary directory to simulate a project
    let dir = tempfile::tempdir().unwrap();
    let dir_path = dir.path();

    // Create a more complex React component structure
    let entry_path = dir_path.join("App.tsx");
    let parent_path = dir_path.join("ParentComponent.tsx");
    let child_path = dir_path.join("ChildComponent.tsx");

    // Write the entry file
    let mut entry_file = File::create(&entry_path).unwrap();
    write!(
        entry_file,
        r#"
                import React from 'react';
                import ParentComponent from './ParentComponent';

                function App() {{
                    return <ParentComponent />;
                }}

                export default App;
            "#
    )
        .unwrap();

    // Write the parent component file
    let mut parent_file = File::create(&parent_path).unwrap();
    write!(
        parent_file,
        r#"
                import React from 'react';
                import ChildComponent from './ChildComponent';

                const ParentComponent = () => {{
                    return <ChildComponent />;
                }};

                export default ParentComponent;
            "#
    )
        .unwrap();

    // Write the child component file
    let mut child_file = File::create(&child_path).unwrap();
    write!(
        child_file,
        r#"
                import React from 'react';

                const ChildComponent = () => {{
                    return <div>Child Component</div>;
                }};

                export default ChildComponent;
            "#
    )
        .unwrap();

    // Now, let's traverse the project
    let traverser = ProjectTraverser::new();
    let (result, _bla) = traverser.traverse(&entry_path).unwrap();

    // Ensure all files were visited
    let mut expected_files = HashSet::new();
    expected_files.insert(entry_path.canonicalize().unwrap());
    expected_files.insert(parent_path.canonicalize().unwrap());
    expected_files.insert(child_path.canonicalize().unwrap());

    assert_eq!(result, expected_files);

    // Optionally, capture and assert the correct graph output.
}

#[test]
fn test_project_traverser_graph_output() {
    // Create a temporary directory to simulate a project
    let dir = tempfile::tempdir().unwrap();
    let dir_path = dir.path();

    // Create a simple React component structure
    let entry_path = dir_path.join("App.tsx");
    let child_path = dir_path.join("ChildComponent.tsx");

    // Write the entry file
    let mut entry_file = File::create(&entry_path).unwrap();
    write!(
        entry_file,
        r#"
                import React from 'react';
                import ChildComponent from './ChildComponent';

                const App = () => {{
                    return <ChildComponent />;
                }}

                export default App;
            "#
    )
        .unwrap();

    // Write the child component file
    let mut child_file = File::create(&child_path).unwrap();
    write!(
        child_file,
        r#"
                import React from 'react';

                const ChildComponent = () => {{
                    return <div>Child Component</div>;
                }};

                export default ChildComponent;
            "#
    )
        .unwrap();

    // Traverse the project and capture the graph output
    let traverser = ProjectTraverser::new();
    let (_bla, graph) = traverser.traverse(&entry_path).unwrap();

    // Manually create the expected DOT graph output
    let mut expected_graph = DiGraphMap::new();
    expected_graph.add_edge("App", "ChildComponent", ());

    let expected_dot = format!("{:?}", Dot::with_config(&expected_graph, &[Config::EdgeNoLabel]));

    // Assert that the output matches the expected graph
    assert_eq!(graph, expected_dot);
}

#[test]
fn test_project_traverser_graph_output_with_unique_components() {
    // Create a temporary directory to simulate a project
    let dir = tempfile::tempdir().unwrap();
    let dir_path = dir.path();

    // Create a simple React component structure
    let entry_path = dir_path.join("App.tsx");

    let component_dir = dir_path.join("components");
    std::fs::create_dir(&component_dir).unwrap();
    let barrel_path = component_dir.join("index.ts");
    let child_path = component_dir.join("ChildComponent.tsx");

    // Write the entry file
    let mut entry_file = File::create(&entry_path).unwrap();
    write!(
        entry_file,
        r#"
                import React from 'react';
                import {{ ChildComponent }} from './components';

                const App = () => {{
                    return <ChildComponent />;
                }}

                export default App;
            "#
    )
        .unwrap();

    // Write the barrel file
    let mut barrel_file = File::create(&barrel_path).unwrap();
    write!(
        barrel_file,
        r#"
                export {{ ChildComponent }} from './ChildComponent';
            "#
    )
        .unwrap();

    // Write the child component file
    let mut child_file = File::create(&child_path).unwrap();
    write!(
        child_file,
        r#"
                import React from 'react';

                export const ChildComponent = () => {{
                    return <div>Child Component</div>;
                }};

                export default ChildComponent;
            "#
    )
        .unwrap();

    // Traverse the project and capture the graph output
    let traverser = ProjectTraverser::new();
    let (_bla, graph) = traverser.traverse(&entry_path).unwrap();

    // Manually create the expected DOT graph output
    let mut expected_graph = DiGraphMap::new();
    expected_graph.add_edge("App", "ChildComponent", ());

    let expected_dot = format!("{:?}", Dot::with_config(&expected_graph, &[Config::EdgeNoLabel]));

    // Assert that the output matches the expected graph
    assert_eq!(graph, expected_dot);
}
