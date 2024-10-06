<p align="center">
<img src="https://github.com/user-attachments/assets/2a34f4c2-fcfe-420f-823f-bc0f816aebf7" height="200">
</p>

<h1 align="center">
Spinne
</h1>
<p align="center">
Spins a web of components and analyzes component/prop usage in your react project
<p>

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

## Options

| Option | Description | Options | Default |
| --- | --- | --- | --- |
| `-e, --entry <path>` | entry point directory | Path | current directory |
| `-o, --output <format>` | define the output type | `file`, `console` | file |
| `-i, --ignore <path>` | define ignored folders | comma separated glob patterns | `**/node_modules/**,**/dist/**,**/build/**` |