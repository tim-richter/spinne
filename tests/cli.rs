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
    fs::write(root.join("package.json"), "{}").unwrap();

    temp_dir
}

#[test]
fn test_cli_with_default_output() {
    let temp_dir = create_mock_project(vec![
        ("src/components/Button.tsx", "export function Button() { return <button>Click me</button>; }"),
    ]);
    let mut cmd = Command::cargo_bin("spinne").unwrap();

    cmd.arg("-e")
        .arg(temp_dir.path().join("src"))
        .arg("-o")
        .arg(temp_dir.path().join("spinnen-netz.json"))
        .assert()
        .success()
        .stdout(predicate::str::contains("Writing report to:"));

    // Check if the output file is created
    assert!(temp_dir.path().join("spinnen-netz.json").exists());
}

#[test]
fn test_cli_with_console_output() {
    let temp_dir = create_mock_project(vec![
        ("src/components/Button.tsx", "export function Button() { return <button>Click me</button>; }"),
    ]);
    let mut cmd = Command::cargo_bin("spinne").unwrap();

    cmd.arg("-e")
        .arg(temp_dir.path().join("src"))
        .arg("-o")
        .arg(temp_dir.path().join("spinnen-netz.json"))
        .arg("-f")
        .arg("console")
        .assert()
        .success()
        .stdout(predicate::str::contains("Printing report to console:"))
        .stdout(predicate::str::contains("Button"));

    // Check that no output file is created
    assert!(!temp_dir.path().join("spinnen-netz.json").exists());
}

#[test]
fn test_cli_with_ignore_option() {
    let temp_dir = create_mock_project(vec![
        ("src/components/Button.tsx", "export function Button() { return <button>Click me</button>; }"),
        ("src/pages/Home.tsx", "import { Header } from '../components/Header'; export function Home() { return <div><Header /><main>Welcome</main></div>; }"),
    ]);
    let mut cmd = Command::cargo_bin("spinne").unwrap();

    cmd.arg("-e")
        .arg(temp_dir.path().join("src"))
        .arg("-i")
        .arg("**/components/**")
        .arg("-f")
        .arg("console")
        .assert()
        .success()
        .stdout(predicate::str::contains("Home"))
        .stdout(predicate::str::contains("Button").not());
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
        ("src/components/Button.tsx", "export function Button() { return <button>Click me</button>; }"),
        ("src/components/Header.tsx", "import { Button } from './Button'; export function Header() { return <header><Button /></header>; }"),
        ("src/pages/Home.tsx", "import { Header } from '../components/Header'; export function Home() { return <div><Header /><main>Welcome</main></div>; }"),
        ("src/index.tsx", "import { Home } from './pages/Home'; export function App() { return <Home />; }"),
    ]);
    let mut cmd = Command::cargo_bin("spinne").unwrap();

    cmd.arg("-e")
        .arg(temp_dir.path().join("src"))
        .arg("-i")
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