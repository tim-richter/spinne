use spinne_core::ProjectTraverser;
use spinne_logger::Logger;
use std::fs;
use tempfile::TempDir;

fn create_mock_project(files: Vec<(&str, &str)>) -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    // Create a nested directory structure
    fs::create_dir_all(root.join("src/components")).unwrap();
    fs::create_dir_all(root.join("src/components/Button")).unwrap();
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

    let mut traverser = ProjectTraverser::new(&temp_dir.path());
    let component_graph = traverser
        .traverse(
            &temp_dir.path().join("src"),
            &vec![],
            &vec!["**/*.tsx".to_string()],
        )
        .unwrap();

    assert!(
        component_graph.has_component("MyComponent", &temp_dir.path().join("src/MyComponent.tsx"))
    );
    assert!(component_graph.has_component(
        "MyComponent2",
        &temp_dir.path().join("src/MyComponent2.tsx")
    ));
    assert!(
        component_graph.has_component("Button", &temp_dir.path().join("src/components/Button.tsx"))
    );
    assert!(component_graph.graph.node_count() == 3);

    let my_component_index = component_graph
        .get_component("MyComponent", &temp_dir.path().join("src/MyComponent.tsx"))
        .unwrap();
    let my_component2_index = component_graph
        .get_component(
            "MyComponent2",
            &temp_dir.path().join("src/MyComponent2.tsx"),
        )
        .unwrap();
    let button_index = component_graph
        .get_component("Button", &temp_dir.path().join("src/components/Button.tsx"))
        .unwrap();

    assert!(component_graph
        .graph
        .contains_edge(my_component_index, button_index));
    assert!(component_graph.graph.edges(my_component_index).count() == 1);
    assert!(component_graph
        .graph
        .contains_edge(my_component2_index, button_index));
    assert!(component_graph.graph.edges(my_component2_index).count() == 1);
    assert!(component_graph.graph.edges(button_index).count() == 0);
}

#[test]
fn should_resolve_imports_with_relative_and_tsconfig_paths_correctly() {
    let temp_dir = create_mock_project(vec![
      ("src/components/index.ts", "export { Button } from './Button';"),
      ("src/components/Button.tsx", "export function Button() { return <button>Click me</button>; }"),
      ("src/MyComponent.tsx", "import { Button } from './components'; function MyComponent() { return <Button />; }"),
      ("src/MyComponent2.tsx", "import { Button } from './components/Button'; function MyComponent2() { return <Button />; }"),
      ("src/MyComponent3.tsx", "import { Button } from '@components/Button'; function MyComponent3() { return <Button />; }"),
    ]);

    let tsconfig_path = temp_dir.path().join("tsconfig.json");
    let tsconfig_content = format!(
        r#"
    {{
        "compilerOptions": {{
            "baseUrl": "{}",
            "paths": {{
                "@components/*": ["src/components/*"]
            }}
        }}
    }}
"#,
        temp_dir.path().to_string_lossy()
    );
    fs::write(tsconfig_path, tsconfig_content).unwrap();

    let mut traverser = ProjectTraverser::new(&temp_dir.path());
    let component_graph = traverser
        .traverse(
            &temp_dir.path().join("src"),
            &vec![],
            &vec!["**/*.tsx".to_string()],
        )
        .unwrap();

    assert!(
        component_graph.has_component("MyComponent", &temp_dir.path().join("src/MyComponent.tsx"))
    );
    assert!(component_graph.has_component(
        "MyComponent2",
        &temp_dir.path().join("src/MyComponent2.tsx")
    ));
    assert!(component_graph.has_component(
        "MyComponent3",
        &temp_dir.path().join("src/MyComponent3.tsx")
    ));

    assert!(
        component_graph.has_component("Button", &temp_dir.path().join("src/components/Button.tsx"))
    );
    println!("{:?}", component_graph.graph);
    assert!(component_graph.graph.node_count() == 4);

    let my_component_index = component_graph
        .get_component("MyComponent", &temp_dir.path().join("src/MyComponent.tsx"))
        .unwrap();
    let my_component2_index = component_graph
        .get_component(
            "MyComponent2",
            &temp_dir.path().join("src/MyComponent2.tsx"),
        )
        .unwrap();
    let button_index = component_graph
        .get_component("Button", &temp_dir.path().join("src/components/Button.tsx"))
        .unwrap();

    assert!(component_graph
        .graph
        .contains_edge(my_component_index, button_index));
    assert!(component_graph.graph.edges(my_component_index).count() == 1);
    assert!(component_graph
        .graph
        .contains_edge(my_component2_index, button_index));
    assert!(component_graph.graph.edges(my_component2_index).count() == 1);
    assert!(component_graph.graph.edges(button_index).count() == 0);
}

#[test]
fn should_resolve_imports_with_two_different_tsconfig_paths_to_the_same_file() {
    Logger::set_level(2);

    let temp_dir = create_mock_project(vec![
        ("src/components/index.ts", "export { Button, Button2 } from './Button';"),
        ("src/components/Button/index.ts", "export { Button } from './Button'; export { Button2 } from './Button2';"),
        ("src/components/Button/Button2.tsx", "export function Button2() { return <button>Click me</button>; }"),
        ("src/components/Button/Button.tsx", "export function Button() { return <button>Click me</button>; }"),

        ("src/MyComponent.tsx", "import { Button } from '@components/Button'; function MyComponent() { return <Button />; }"),
        ("src/MyComponent2.tsx", "import { Button2 } from '@components'; function MyComponent2() { return <Button2 />; }"),
        ("src/MyComponent3.tsx", "import { Button } from '@components/Button/Button'; function MyComponent3() { return <Button />; }"),
    ]);

    let tsconfig_path = temp_dir.path().join("tsconfig.json");
    let tsconfig_content = format!(
        r#"
    {{
        "compilerOptions": {{
            "baseUrl": "{}",
            "paths": {{
                "@components": ["src/components"],
                "@components/*": ["src/components/*"]
            }}
        }}
    }}
"#,
        temp_dir.path().to_string_lossy()
    );
    fs::write(tsconfig_path, tsconfig_content).unwrap();

    let mut traverser = ProjectTraverser::new(&temp_dir.path());
    let component_graph = traverser
        .traverse(
            &temp_dir.path().join("src"),
            &vec![],
            &vec!["**/*.tsx".to_string()],
        )
        .unwrap();

    assert!(
        component_graph.has_component("MyComponent", &temp_dir.path().join("src/MyComponent.tsx"))
    );
    assert!(component_graph.has_component(
        "MyComponent2",
        &temp_dir.path().join("src/MyComponent2.tsx")
    ));
    assert!(component_graph.has_component(
        "MyComponent3",
        &temp_dir.path().join("src/MyComponent3.tsx")
    ));
    assert!(component_graph.has_component(
        "Button",
        &temp_dir.path().join("src/components/Button/Button.tsx")
    ));
    assert!(component_graph.has_component(
        "Button2",
        &temp_dir.path().join("src/components/Button/Button2.tsx")
    ));
    println!("{:?}", component_graph.graph);
    assert!(component_graph.graph.node_count() == 5);
}
