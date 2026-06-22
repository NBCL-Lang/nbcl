# Changelog

All notable changes to `nbcl` are documented here.

This changelog follows the [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) format,
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.5.1] - 2026-06-10

### Fixed

- Context not being returned if error in `evaluate_ast_for_ctx`.

## [0.5.0] - 2026-06-07

### Added

- `get_current_file` function to Context.

### Fixed

- Not being able to escape string interpolation.
- Not able to use if statements inside node body.
- Optional params being required inside component.

## [0.4.7] - 2026-05-31

### Fixed

- `unqoute` function greedly truncating both start and end.

## [0.4.6] - 2026-05-29

### Fixed

- Components being inaccessible within that module itself.
- Statements not extending nodes within imported modules.

## [0.4.5] - 2026-05-28

### Fixed

- Keyword boundary problem with snake idents.

## [0.4.4] - 2026-05-28

### Fixed

- Lambdas letting only first statement pass through.

## [0.4.3] - 2026-05-28

### Fixed

- Context structure lacking derive Clone.

## [0.4.2] - 2026-05-26

### Fixed

- Not being able to call methods of LibraryItem

## [0.4.1] - 2026-05-25

### Fixed

- Not able to define node in if, while, for, etc.
- Not being able to return node in lambda.
- Node not expanding when returned from function at top level.

## [0.4.0] - 2026-05-23

### Added

- Internal Object type to register data that user cant interact with.
- String interpolation support for variables.

### Fixed

- Strict prop validation throwing error on matching types.
- Not checking child count on native nodes.
- Not throwing error when properties were passed to a component that expected none.
- Wasm using outdated API.
- No default feature build failing.

## [0.3.4] - 2026-05-21

### Fixed

- Maps requiring `,` which breaks uniformity with nodes.

## [0.3.3] - 2026-05-21

### Fixed

- `call_function` requiring Context to be owned.

## [0.3.2] - 2026-05-20

### Fixed

- Context structure being inaccessible to public.

## [0.3.1] - 2026-05-20

### Fixed

- `NbclEngine.call_function` not finding functions.

## [0.3.0] - 2026-05-20

### Added

- Feature to resolve functions/lambdas even after evaluation.

### Changed

- Replaced `local` and `global` with `let` and `const`.
- Import module statement to optionally import components.
- `no-module-imports` and `no-lib-imports` feature to be additive instead.

### Fixed

- Errors from `as_int` saying its `as_float`.

### Remove

- Type hints.

## [0.2.0] - 2026-05-15

### Added

- Lambda support.
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
- snake_ident to support snake cases in ALL_CAPS.

### Fixed

- Not being able to explicitly return nodes in functions.
- Not being able to implicitly return expressions in functions.
- Parser producing incorrect errors when dealing with "in" inside for loops.
- Broken match conditional statement.

## [0.1.0] - 2026-05-12

- Initial Release.
