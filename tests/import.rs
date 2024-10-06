use spinne::ProjectTraverser;
use std::fs;
use tempfile::TempDir;

fn create_mock_project(files: Vec<(&str, &str)>) -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create a nested directory structure
    fs::create_dir_all(root.join("src/components")).unwrap();
    fs::create_dir_all(root.join("src/pages")).unwrap();

    // Create mock .tsx files
    for (path, content) in files {
        let file_path = root.join(path);
        fs::write(file_path, content).unwrap();
    }

    // Create non-tsx files
    fs::write(root.join("README.md"), "# Mock Project").unwrap();
    fs::write(root.join("package.json"), "{}").unwrap();

    temp_dir
}

#[test]
fn should_resolve_imports_correctly_with_multiple_files() {
    let temp_dir = create_mock_project(vec![
      ("src/components/index.ts", "export { Button } from './Button';"),
      ("src/components/Button.tsx", "export function Button() { return <button>Click me</button>; }"),
      ("src/MyComponent.tsx", "import { Button } from './components'; function MyComponent() { return <Button />; }"),
      ("src/MyComponent2.tsx", "import { Button } from './components/Button'; function MyComponent2() { return <Button />; }"),
    ]);
    
    let mut traverser = ProjectTraverser::new();
    let component_graph = traverser.traverse(&temp_dir.path().join("src"), &vec![]).unwrap();

    assert!(component_graph.has_component("MyComponent", &temp_dir.path().join("src/MyComponent.tsx")));
    assert!(component_graph.has_component("MyComponent2", &temp_dir.path().join("src/MyComponent2.tsx")));
    assert!(component_graph.has_component("Button", &temp_dir.path().join("src/components/Button.tsx")));
    assert!(component_graph.graph.node_count() == 3);

    let my_component_index = component_graph.get_component("MyComponent", &temp_dir.path().join("src/MyComponent.tsx")).unwrap();
    let my_component2_index = component_graph.get_component("MyComponent2", &temp_dir.path().join("src/MyComponent2.tsx")).unwrap();
    let button_index = component_graph.get_component("Button", &temp_dir.path().join("src/components/Button.tsx")).unwrap();

    assert!(component_graph.graph.contains_edge(my_component_index, button_index));
    assert!(component_graph.graph.edges(my_component_index).count() == 1);
    assert!(component_graph.graph.contains_edge(my_component2_index, button_index));
    assert!(component_graph.graph.edges(my_component2_index).count() == 1);
    assert!(component_graph.graph.edges(button_index).count() == 0);
}