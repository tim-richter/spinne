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

Spinne is a CLI Tool that analyzes react projects and creates a component graph from all components that are used in it. This allows you to make some educated guesses about:
- component usage
- component relationships

## Example

```json
{
  "nodes": [
    {
      "name": "ButtonGroup",
      "file_path": "@smartclip/common-ui-components/src/components/Button/ButtonGroup.tsx",
      "prop_usage": {}
    },
    {
      "name": "MyComponent",
      "file_path": "@smartclip/common-ui-components/src/components/MyComponent.tsx",
      "prop_usage": {}
    }
  ],
  "edges": [
    [
      0,
      1
    ]
  ]
}
```

For the graph, we are using a directed adjacency graph, which means relationships between components are representated by edges, or weights.
A '0' to '1' edge, would mean, that the first node, "ButtonGroup", uses the second node, "MyComponent", in the code of our project.

## Installation

Spinne is a command line tool written in rust, so you need to have rust/cargo installed.

```bash
cargo install spinne
```

## Usage

To scan for components in your current directory:

```bash
spinne
```

This command will output the results in a file 'spinne-report.json' by default.
If you want to output it directly to the console you can use `-o console`:

```bash
spinne -o console
```

To output the results in a html format with a visualization of the component graph:

```bash
spinne -f html
```

## Options

| Option | Description | Options | Default |
| --- | --- | --- | --- |
| `-e, --entry <path>` | entry point directory | Path | current directory (./) |
| `-f, --format <format>` | define the output format | `file`, `console`, `html` | file |
| `-i, --ignore <path>` | define ignored folders | comma separated glob patterns | `**/node_modules/**,**/dist/**,**/build/**` |
| `--file-name <file-name>` | define the output file name | String | `spinne-report` |
| `-l, --log-level <log-level>` | define the log level | the amount of -l used will define the log level | 0 |
| `--include <include>` | define a glob pattern to include | comma separated glob patterns | `**/*.tsx` |
| `--exclude <exclude>` | define a glob pattern to exclude | comma separated glob patterns | `**/node_modules/**,**/dist/**,**/build/**,**/*.stories.tsx,**/*.test.tsx` |
