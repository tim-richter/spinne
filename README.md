<p align="center">
<img src="https://github.com/user-attachments/assets/2a34f4c2-fcfe-420f-823f-bc0f816aebf7" height="200">
</p>

<h1 align="center">
Spinne
</h1>
<p align="center">
Spins a web of components and analyzes prop usage, adoption and more!
<p>

## Installation

```bash
npm i -g spinne
```

## Usage

To scan for components in your current directory:

```bash
spinne scan
```

This command will output the results in a file 'scan-data.json'.
If you want to output it directly to the console you can use `-o console`:

```bash
spinne scan -o console
```

## Options

- `-d, --directory <path>`: scan from a different directory
- `-o, --output <format>`: define the output format 
