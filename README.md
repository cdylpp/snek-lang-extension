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

Published builds include the Snek language server. Users do not need Rust, Cargo, or the Snek compiler repository installed.

Supported extension targets:

- macOS Apple Silicon: `darwin-arm64`
- macOS Intel: `darwin-x64`
- Linux x64: `linux-x64`
- Linux ARM64: `linux-arm64`
- Windows x64: `win32-x64`

## Development

The language server is built with Rust. Before running the extension locally, build the release server:

```sh
npm run build-server
```

For platform-specific VSIX builds, build and package the supported targets:

```sh
npm run package:targets
```

Cross-platform builds require the matching Rust target standard libraries and linkers. The macOS targets can be built on macOS; Linux and Windows targets should be built on matching CI runners or on a machine configured with compatible cross linkers.

The extension loads the bundled server from `<target>/server/bin/snek-lsp` on macOS/Linux or `<target>/server/bin/snek-lsp.exe` on Windows.

## Extension Settings

This extension does not currently contribute user-configurable settings.

It does set default editor behavior for Snek files:

- Enables semantic highlighting for `snek-lang`.
- Italicizes function declarations and parameter declarations when semantic tokens are available.

## Known Issues

- Language support is intentionally basic and focused on early compiler-learning workflows.
