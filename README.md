# shut

A minimal CLI tool to kill a process listening on a port.

## Usage

Kill process listening on port 6969

```shell
shut 6969
```

# Alternatives

Linux / Mac OS X:
```shell
kill -9 $(lsof -t -i:6969)
```