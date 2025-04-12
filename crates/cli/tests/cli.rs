use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;
use serde_json::Value;

pub fn create_mock_project(files: &Vec<(&str, &str)>) -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create mock .tsx files
    for (path, content) in files {
        // create directories before creating files
        let file_path = root.join(path);
        if let Some(parent) = file_path.parent() {
            if parent != root {
                fs::create_dir_all(parent).unwrap();
            }
        }
        fs::write(file_path, content).unwrap();
    }

    temp_dir
}

#[test]
fn test_cli_with_default_output() {
    let temp_dir = create_mock_project(&vec![(
        "src/components/Button.tsx",
        "export const Button = () => { return <button>Click me</button>; }",
    )]);
    let mut cmd = Command::cargo_bin("spinne").unwrap();

    cmd.current_dir(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Writing report to:"));

    // Check if the output file is created
    assert!(temp_dir.path().join("spinne-report.json").exists());
}

#[test]
fn test_cli_with_console_output() {
    let temp_dir = create_mock_project(&vec![
        (".git/HEAD", "ref: refs/heads/main"),
        ("package.json", r#"{"name": "mock-project"}"#),
        (
            "src/components/Button.tsx",
            "export const Button = () => { return <button>Click me</button>; }",
        ),
    ]);
    let mut cmd = Command::cargo_bin("spinne").unwrap();

    cmd.current_dir(temp_dir.path())
        .arg("-f")
        .arg("console")
        .assert()
        .success()
        .stdout(predicate::str::contains("Printing report to console:"))
        .stdout(predicate::str::contains("Button"));

    // Check that no output file is created
    assert!(!temp_dir.path().join("spinne-report.json").exists());
}

#[test]
fn test_cli_with_ignore_option() {
    let temp_dir = create_mock_project(&vec![
        (".git/HEAD", "ref: refs/heads/main"),
        ("package.json", r#"{"name": "mock-project"}"#),
        ("src/components/Button.tsx", "export const Button = () => { return <button>Click me</button>; }"),
        ("src/pages/Home.tsx", "import { Header } from '../components/Header'; export const Home = () => { return <div><Header /><main>Welcome</main></div>; }"),
        ("src/components/Header.tsx", "export const Header = () => { return <header>Header</header>; }"),
    ]);
    let mut cmd = Command::cargo_bin("spinne").unwrap();

    cmd.current_dir(temp_dir.path())
        .arg("--exclude")
        .arg("**/components/**")
        .arg("-f")
        .arg("console")
        .assert()
        .success()
        .stdout(predicate::str::contains("Home"))
        .stdout(predicate::str::contains("Header"));
}

#[test]
fn test_cli_with_nonexistent_directory() {
    let mut cmd = Command::cargo_bin("spinne").unwrap();

    cmd.arg("-e")
        .arg("/path/to/nonexistent/directory")
        .assert()
        .failure()
        .stderr(predicate::str::contains("No such file or directory"));
}

#[test]
fn test_cli_with_ignore_multiple_directories() {
    let temp_dir = create_mock_project(&vec![
        (".git/HEAD", "ref: refs/heads/main"),
        ("package.json", r#"{"name": "mock-project"}"#),
        ("src/components/Button.tsx", "export const Button = () => { return <button>Click me</button>; }"),
        ("src/components/Header.tsx", "import { Button } from './Button'; export const Header = () => { return <header><Button /></header>; }"),
        ("src/pages/Home.tsx", "import { Header } from '../components/Header'; export const Home = () => { return <div><Header /><main>Welcome</main></div>; }"),
        ("src/index.tsx", "import { Home } from './pages/Home'; export const App = () => { return <Home />; }"),
    ]);
    let mut cmd = Command::cargo_bin("spinne").unwrap();

    cmd.current_dir(temp_dir.path())
        .arg("--exclude")
        .arg("**/components/**,**/pages/**")
        .arg("-f")
        .arg("console")
        .assert()
        .success()
        .stdout(predicate::str::contains("Button").not())
        .stdout(predicate::str::contains("Header").not())
        .stdout(predicate::str::contains("Home"))
        .stdout(predicate::str::contains("App"));
}

#[test]
fn test_cli_with_include_option() {
    let temp_dir = create_mock_project(&vec![
        (".git/HEAD", "ref: refs/heads/main"),
        ("package.json", r#"{"name": "mock-project"}"#),
        (
        "src/components/Button.tsx",
        "export const Button = () => { return <button>Click me</button>; }",
    ), (
        "src/pages/Home.tsx",
        "import { Header } from '../components/Header'; export const Home = () => { return <div><Header /><main>Welcome</main></div>; }",
    ), (
        "src/components/Header.tsx",
        "export const Header = () => { return <header>Header</header>; }",
    )
    ]);
    let mut cmd = Command::cargo_bin("spinne").unwrap();

    cmd.current_dir(temp_dir.path())
        .arg("--include")
        .arg("**/pages/**/*.tsx")
        .arg("-f")
        .arg("console")
        .assert()
        .success()
        .stdout(predicate::str::contains("Home"))
        .stdout(predicate::str::contains("Button").not());
}

#[test]
fn test_cli_with_html_output() {
    let temp_dir = create_mock_project(&vec![
        (".git/HEAD", "ref: refs/heads/main"),
        ("package.json", r#"{"name": "mock-project"}"#),
        (
        "src/components/Button.tsx",
        "export const Button = () => { return <button>Click me</button>; }",
    ), (
        "src/pages/Home.tsx",
        "import { Header } from '../components/Header'; export const Home = () => { return <div><Header /><main>Welcome</main></div>; }",
    ),
    (
        "src/index.tsx",
        "import { Home } from './pages/Home'; export const App = () => { return <Home />; }",
    ),
    ("src/components/Header.tsx", "export const Header = () => { return <header>Header</header>; }"),
    ]);
    let mut cmd = Command::cargo_bin("spinne").unwrap();

    cmd.current_dir(temp_dir.path())
        .arg("-f")
        .arg("html")
        .assert()
        .success()
        .stdout(predicate::str::contains("Writing report to:"));

    // Check if the output file is created
    assert!(temp_dir.path().join("spinne-report.html").exists());
}

#[test]
fn test_cli_with_json_output() {
    let temp_dir = create_mock_project(&vec![
        (".git/HEAD", "ref: refs/heads/main"),
        ("package.json", r#"{"name": "mock-project"}"#),
        (
            "src/components/Button.tsx",
            "export const Button = () => { return <button>Click me</button>; }",
        ),
        (
            "src/pages/Home.tsx",
            "import { Button } from '../components/Button'; export const Home = () => { return <div><Button /><main>Welcome</main></div>; }",
        ),
    ]);
    let mut cmd = Command::cargo_bin("spinne").unwrap();

    cmd.current_dir(temp_dir.path())
        .arg("-f")
        .arg("file")
        .assert()
        .success()
        .stdout(predicate::str::contains("Writing report to:"));

    // Check if the output file is created
    let output_path = temp_dir.path().join("spinne-report.json");
    assert!(output_path.exists());

    // Read and parse the JSON output
    let json_content = fs::read_to_string(output_path).unwrap();
    let json: Value = serde_json::from_str(&json_content).unwrap();

    // Verify JSON structure
    assert!(json.is_array());
    let project = &json[0];
    assert_eq!(project["name"], "mock-project");

    let graph = &project["graph"];
    assert!(graph.is_object());
    assert!(graph.get("components").is_some());
    assert!(graph.get("edges").is_some());

    let components = graph["components"].as_array().unwrap();
    let edges = graph["edges"].as_array().unwrap();

    // Verify component structure
    assert!(!components.is_empty());
    let component = &components[0];
    assert!(component.get("id").is_some());
    assert!(component.get("name").is_some());
    assert!(component.get("path").is_some());
    assert!(component.get("props").is_some());
    assert!(component.get("project").is_some());

    // Verify edge structure (should have one edge from Home to Button)
    assert_eq!(edges.len(), 1);
    let edge = &edges[0];
    assert!(edge.get("from").is_some());
    assert!(edge.get("to").is_some());

    // Verify specific components exist
    let component_names: Vec<&str> = components
        .iter()
        .map(|c| c.get("name").unwrap().as_str().unwrap())
        .collect();
    assert!(component_names.contains(&"Button"));
    assert!(component_names.contains(&"Home"));

    // Verify the edge is from Home to Button
    let button_id = components
        .iter()
        .find(|c| c["name"] == "Button")
        .unwrap()["id"]
        .as_u64()
        .unwrap();
    let home_id = components
        .iter()
        .find(|c| c["name"] == "Home")
        .unwrap()["id"]
        .as_u64()
        .unwrap();
    assert_eq!(edge["from"].as_u64().unwrap(), home_id);
    assert_eq!(edge["to"].as_u64().unwrap(), button_id);
}
