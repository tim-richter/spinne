use assert_cmd::Command;
use predicates::prelude::*;
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
    fs::write(root.join("package.json"), r#"{"name": "mock-project"}"#).unwrap();

    temp_dir
}

#[test]
fn test_cli_with_default_output() {
    let temp_dir = create_mock_project(vec![(
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
    let temp_dir = create_mock_project(vec![(
        "src/components/Button.tsx",
        "export const Button = () => { return <button>Click me</button>; }",
    )]);
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
    let temp_dir = create_mock_project(vec![
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
    let temp_dir = create_mock_project(vec![
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
    let temp_dir = create_mock_project(vec![(
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
    let temp_dir = create_mock_project(vec![(
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
