# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `--commit` flag to also restart the command when git HEAD changes. (#36)

## [0.3.4] - 2026-05-17

### Fixed

- Mark basic example doctest as `no_run` to prevent clap exit failure.
- Correct dependency version in README (`"0.3.3"` → `"0.3"`).
- Add missing struct body to troubleshooting doctest snippets in README.

## [0.3.3] - 2026-04-25

### Added

- `WatchLock` type to coordinate rebuilds with external readers (#34)
- Glob pattern support for excluded paths (#32)

## [0.3.2] - 2025-05-11

### Changed

- Improved `handle_event` conditions

## [0.3.1] - 2025-04-16

### Fixed

- Fixed infinite lock
- Fixed GitHub Actions workflow not testing properly

## [0.3.0] - 2025-04-16

### Added

- `exec` and `shell` options to the CLI (#28)

### Changed

- Updated dependencies and workflow (#29)

[Unreleased]: https://github.com/rustminded/xtask-watch/compare/v0.3.3...HEAD
[0.3.3]: https://github.com/rustminded/xtask-watch/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/rustminded/xtask-watch/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/rustminded/xtask-watch/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/rustminded/xtask-watch/releases/tag/v0.3.0
