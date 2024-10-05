<p align="center">
<img src="https://github.com/user-attachments/assets/2a34f4c2-fcfe-420f-823f-bc0f816aebf7" height="200">
</p>

<h1 align="center">
Spinne
</h1>
<p align="center">
Spins a web of components and analyzes prop usage, adoption etc
<p>

## Installation

```bash
cargo install spinne
```

## Usage

To scan for components in your current directory:

```bash
spinne -e ./src
```

This command will output the results in a file 'scan-data.json'.
If you want to output it directly to the console you can use `-o console`:

```bash
spinne -e ./src -o console
```

## Options

- `-e, --entry <path>`: entry point file or directory
- `-o, --output <format>`: define the output format 
- `-i, --ignore <path>`: define ignored folders. this is set to a reasonable default, but if
you need more control over the scanned `.tsx` files, you might need to use this. paths will get matched
against all subpaths. You can define multiple via `-i fixtures dist`, or `-i fixtures -i dist`
