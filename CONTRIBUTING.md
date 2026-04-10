# Contributing to wxml-parser-rust

Thanks for your interest in contributing! Here's how to get started.

## Development Setup

1. **Prerequisites**: Node.js >= 18, Rust stable toolchain
2. **Install dependencies**:

   ```bash
   npm install
   ```

3. **Build**:

   ```bash
   npm run build:debug
   ```

4. **Test**:

   ```bash
   npm test
   ```

5. **Benchmark**:

   ```bash
   npm run bench
   ```

## Making Changes

- Rust core: edit `crates/wxml-parser-core/`
- Node.js bindings: edit `crates/wxml-parser-napi/` and `loader.js`
- Tests: add/update files in `tests/`

### Commit Style

Use concise, descriptive commit messages. Prefix with conventional commit types when appropriate:

- `feat:` new feature
- `fix:` bug fix
- `refactor:` code restructuring
- `test:` test changes
- `docs:` documentation
- `chore:` build/CI/tooling

### Pull Requests

1. Fork the repository
2. Create a feature branch from `main`
3. Make your changes with clear commits
4. Ensure tests pass (`npm test` and `cargo test -p wxml-parser`)
5. Open a PR against `main` with a clear description

### Version Bumping

This project publishes to both npm and crates.io. Version numbers must stay in sync across:

- `package.json` (`version`)
- `crates/wxml-parser-core/Cargo.toml` (`version`)
- `crates/wxml-parser-napi/Cargo.toml` (`version`)

Update `CHANGELOG.md` when changing the version.

## Release Process

Maintainers create releases by pushing a `v*` tag. CI handles building, testing, and publishing automatically.

## Questions?

Feel free to open an issue for discussion.
