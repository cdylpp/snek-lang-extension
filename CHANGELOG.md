# Change Log

All notable changes to the "snek" extension will be documented in this file.

Check [Keep a Changelog](http://keepachangelog.com/) for recommendations on how to structure this file.

## [Unreleased]

## [0.3.1] - 2026-05-22

- Added GitHub Actions to build targets and publish using `vsce`.

## [0.2.0] - 2026-05-22

- Bundled the Rust language server as a release binary loaded from `<target>/server/bin`.
- Added a workspace-local `snek` parser crate so the server build no longer depends on an absolute local path.
- Added platform build and packaging automation for `darwin-arm64`, `darwin-x64`, `linux-x64`, `linux-arm64`, and `win32-x64`.
- Updated packaging excludes so published VSIXs omit Rust source, Cargo build output, tests, scripts, source maps, and local development files.

## [0.1.1] - 2026-05-22

- Initial Snek language support for `.snek` files.
- Added TextMate syntax highlighting for Snek forms, operators, constants, numbers, identifiers, parentheses, and `;;` comments.
- Added language configuration for comments, parentheses, indentation, folding for `fun` and `block` forms, and Snek word boundaries.
- Added a Rust language server that launches from the VS Code extension.
- Added full-document synchronization and parser-backed syntax diagnostics with red squiggles and parser error messages.
- Added hover documentation for Snek keywords, operators, forms, constants, and vector operations.
- Added keyword completion items with Markdown documentation.
- Added semantic highlighting for function declarations, function calls, parameters, keywords, operators, and comments.
- Added starter snippets for functions, lets, if expressions, blocks, loops, and vectors.
- Enabled semantic highlighting by default for Snek files.
