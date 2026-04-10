# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2024-04-08

### Added

- Rust core WXML parser (`wxml-parser` crate)
- Node.js bindings via napi-rs (`parse` / `parseForESLint`)
- Multi-platform prebuilt binaries (linux-x64, win-x64, darwin-x64, darwin-arm64)
- Compatibility test suite against `@wxml/parser`
- Benchmark suite comparing rust-core, napi, and js-parser

[0.1.0]: https://github.com/GeoffZhu/wxml-parser-rust/releases/tag/v0.1.0
