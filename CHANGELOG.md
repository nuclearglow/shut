# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project
adheres to [Semantic Versioning](https://semver.org/).

## [0.2.0] - 2026-11-05

### Breaking

- Exit codes follow `sysexits`: `64` invalid command line, `69` nothing is
  bound to the port, `70` the socket table could not be read, `77`
  insufficient privileges. Previously every failure exited with `1`.
- Requires Rust 1.95 or newer to build.

### Fixed

- `cargo install shut` builds again. 0.1.x depended on `netstat2 0.9`, which
  no longer compiles against current `libc`.
- IPv6-only and UDP listeners are now found. The port is read from the
  kernel's socket table instead of probing with a TCP connect to `127.0.0.1`.
- An established TCP connection that happens to use the port locally is no
  longer treated as a listener.
- A port in use by another user's process is reported as a permission problem
  (`77`, "retry with sudo") instead of "No processes found".
- Kill failures are reported instead of being logged as success.

### Changed

- Replaced the abandoned `heim` and the `tokio` runtime with `netstat2 0.11`
  and `sysinfo 0.40`.
- Every process holding the port is killed once; `shut` never kills itself,
  and re-checks ownership right before signalling.
- Moved to the 2024 edition.

### Added

- `-h`/`--help` and `-V`/`--version`.
- `shut` library crate (`scan_port`, `kill`) with integration tests.
- CI on Linux, macOS and Windows, plus an MSRV check.
- `LICENSE` file.

## [0.1.2] - 2022-01-02

- Documentation updates.

## [0.1.1] - 2021-12-31

- Metadata updates.

## [0.1.0] - 2021-12-31

- Initial release.

[0.2.0]: https://github.com/nuclearglow/shut/compare/4dde501...v0.2.0
[0.1.2]: https://github.com/nuclearglow/shut/commits/4dde501
