# Changelog

All notable changes to `nbcl` are documented here.

This changelog follows the [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) format,
and this project adheres to [Semantic Versioning](https://semver.org/).

## [UNRELEASED]

### Added

- Augmented assignments.
- Maximum recursion depth to avoid stack overflow.
- Better variable handling with FxHashMap for performance.
- Improved parser errors regarding map literals.
- `run_with_config` (wasm) function to evaluate with a custom config.
- `no-module-imports` feature to disable module imports.
- `no-lib-imports` feature to disable library imports.
- `metadata` feature to add spans in resolved tree and node.

### Changed

- Handling of to support calling `to_string(Any)` to be called as `Any.to_string()`.

### Fixed

- Not being able to explicitly return nodes in functions.
- Not being able to implicitly return expressions in functions.
- Parser producing incorrect errors when dealing with "in" inside for loops.
- Broken match conditional statement.

## [0.1.0] - 2026-05-12

- Initial Release.