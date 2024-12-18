<p align="center">
<img src="https://github.com/user-attachments/assets/2a34f4c2-fcfe-420f-823f-bc0f816aebf7" height="200">
</p>

<h1 align="center">
  Spinne
</h1>
<p align="center">
Spins a web of components and analyzes component/prop usage in your react project
</p>
<p align="center">
  <img alt="Crates.io Version" src="https://img.shields.io/crates/v/spinne?style=for-the-badge&label=%20">
</p>

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