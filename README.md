<p align="center">
<img src="https://i.imghippo.com/files/wZYd5959gc.png" height="200">
</p>

<h1 align="center">
  Spinne
</h1>
<p align="center">
Spins a web of component relationships for react projects
</p>
<p align="center">
  <a href="https://crates.io/crates/spinne"><img alt="Crates.io Version" src="https://img.shields.io/crates/v/spinne?style=for-the-badge&label=%20"></a>
</p>

---

> WIP: Spinne is in early development and report structure and cli options are subject to change.

---

Spinne is a CLI Tool, that analyzes react projects, and creates a component graph from all components that are used. This allows you to make some educated guesses about:
- component usage
- component relationships

## Example

Spinne can analyze both single React projects and workspaces containing multiple projects. Here's an example output showing component relationships across multiple projects:

```json
[
  {
    "name": "ui-components",
    "graph": {
      "nodes": [
        {
          "name": "Button",
          "file_path": "packages/ui-components/src/components/Button.tsx",
          "prop_usage": {}
        },
        {
          "name": "ButtonGroup",
          "file_path": "packages/ui-components/src/components/ButtonGroup.tsx",
          "prop_usage": {}
        }
      ],
      "edges": [
        [1, 0]
      ]
    }
  },
  {
    "name": "main-app",
    "graph": {
      "nodes": [
        {
          "name": "LoginForm",
          "file_path": "packages/main-app/src/features/auth/LoginForm.tsx",
          "prop_usage": {}
        }
      ],
      "edges": [[0, 0]]
    }
  }
]
```

For the graph, we use a directed adjacency graph where relationships between components are represented by edges. For example, an edge from ButtonGroup to Button (represented as `[1, 0]` in the edges array) means that ButtonGroup uses the Button component.

## Installation

Spinne is a command line tool written in rust, so the easiest way to install it is via cargo:

```bash
cargo install spinne
```

## Usage

To scan for components in your current directory:

```bash
spinne
```

This command will output the results in a file named 'spinne-report.json' by default.
If you want to output it directly to the console you can use `-f console`:

```bash
spinne -f console
```

To generate an interactive HTML visualization of the component graph:

```bash
spinne -f html
```
This will create 'spinne-report.html' and automatically open it in your default browser.

## Options

| Option | Description | Options | Default |
| --- | --- | --- | --- |
| `-e, --entry <path>` | Entry point directory | Path | current directory (./) |
| `-f, --format <format>` | Output format | `file`, `console`, `html` | `file` |
| `--exclude <patterns>` | Glob patterns to exclude | comma separated patterns | `**/node_modules/**,**/dist/**,**/build/**,**/*.stories.tsx,**/*.test.tsx` |
| `--include <patterns>` | Glob patterns to include | comma separated patterns | `**/*.tsx` |
| `-l` | Verbosity level | Use multiple times (-l, -ll, etc.) | 0 |

## Configuration File

You can also configure Spinne using a `spinne.json` file in your project root. This file allows you to define persistent configuration options that will be used every time you run Spinne.

Example `spinne.json`:
```json
{
  "include": ["**/*.tsx", "**/*.ts"],
  "exclude": ["**/node_modules/**", "**/dist/**", "**/*.test.tsx"]
}
```

### Configuration Options

| Option | Description | Type |
| --- | --- | --- |
| `include` | Array of glob patterns for files to include in the analysis | `string[]` |
| `exclude` | Array of glob patterns for files to exclude from the analysis | `string[]` |

The configuration file options will be merged with any command line arguments you provide. For example, if you specify both exclude patterns in your `spinne.json` and via the `--exclude` flag, both sets of patterns will be used.

## Workspace Support

Spinne automatically detects and analyzes all React projects within a workspace. A project is identified by the presence of both a `package.json` file and a `.git` directory. This means Spinne can:

- Handle projects in subdirectories
- Process multiple independent projects in a directory structure

When analyzing a workspace:
1. Spinne first discovers all valid React projects in the directory tree
2. Each project is analyzed independently
3. Component relationships are tracked per project
4. Results are aggregated in the final output

You can run Spinne at any level of your directory structure:
- Run it in a specific project directory to analyze just that project
- Run it in a workspace root to analyze all contained projects
- Run it in any parent directory to discover and analyze all projects beneath it

```bash
# Analyze a specific project
cd my-project && spinne

# Analyze multiple projects from a parent directory
cd dev/projects && spinne
```
