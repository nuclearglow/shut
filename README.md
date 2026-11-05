# shut

A minimal CLI tool to kill a process listening on a port.

`shut` resolves the port from the kernel's socket table, so it finds IPv4-only,
IPv6-only, dual-stack and UDP listeners, then sends `SIGKILL` to every process
holding that port.

## Usage

Kill the process listening on port 6969:

```shell
shut 6969
```

```text
Kill the process(es) listening on <port>

USAGE:
    shut <port>

OPTIONS:
    -h, --help       Print this help
    -V, --version    Print the version

EXIT CODES:
    0   a process was killed
    64  invalid command line
    69  nothing is bound to the port
    70  the socket table could not be read
    77  insufficient privileges
```

Set `RUST_LOG=debug` for more detail; the default level is `info`.

## Install

```shell
cargo install shut
```

## Notes

- Requires Rust 1.95 or newer to build.
- Processes owned by another user are visible as "port in use, owner not
  visible"; rerun with `sudo` to kill them.
- `shut` never kills itself.

## Alternatives

Linux / macOS:

```shell
kill -9 $(lsof -t -i:6969)
```

## License

MIT
