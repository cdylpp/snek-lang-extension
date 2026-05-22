# Snek Language Support

Snek is a first-principles language development project used for teaching and compiler learning. This extension adds basic Visual Studio Code support for writing `.snek` programs.

## Features

- Syntax highlighting for Snek source files.
- Language activation for files with the `.snek` extension.
- Snippets for common Snek forms, including `fun`, `let`, `if`, `block`, `loop`, and `vec`.
- Parenthesis-aware indentation, folding, and auto-closing pairs.
- `;;` line comments.
- Language server support for parse diagnostics, keyword completions, hover information, and semantic highlighting.

## Requirements

This extension requires Visual Studio Code `^1.120.0`.

The language server is built with Rust. Before running the extension locally, build the server:

```sh
npm run build-server
```

The extension expects the debug language server binary at `server/target/debug/snek-lsp` on macOS/Linux or `server/target/debug/snek-lsp.exe` on Windows.

## Extension Settings

This extension does not currently contribute user-configurable settings.

It does set default editor behavior for Snek files:

- Enables semantic highlighting for `snek-lang`.
- Italicizes function declarations and parameter declarations when semantic tokens are available.

## Known Issues

- The language server must be built locally before the extension can start it.
- Language support is intentionally basic and focused on early compiler-learning workflows.
