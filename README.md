# shut

A small CLI tool to kill a process blocking a port.

## Usage

Kill process listening on port 6969

```shell
shut 6969
```

# Alternatives

Linux:
```shell
kill -9 $(lsof -t -i:8080)
```
