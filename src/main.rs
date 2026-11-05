//! `shut` — kill the process(es) holding a port.
//!
//! ```text
//! shut 6969
//! ```

use std::env;
use std::process;

use env_logger::Env;
use log::{error, info, warn};

const USAGE: &str = "\
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
";

/// What the command line asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Action {
    Kill(u16),
    Help,
    Version,
}

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();

    match parse_args(env::args().skip(1)) {
        Ok(Action::Help) => print!("{USAGE}"),
        Ok(Action::Version) => println!("shut {}", env!("CARGO_PKG_VERSION")),
        Ok(Action::Kill(port)) => run(port),
        Err(message) => {
            error!("{message}");
            eprint!("{USAGE}");
            process::exit(exitcode::USAGE);
        }
    }
}

/// Kill everything holding `port`, exiting with a sysexits code.
fn run(port: u16) {
    let scan = match shut::scan_port(port) {
        Ok(scan) => scan,
        Err(error) => {
            error!("{error}");
            process::exit(exitcode::SOFTWARE);
        }
    };

    if scan.owner_is_hidden() {
        error!("port {port} is in use, but its owner is not visible to this user; retry with sudo");
        process::exit(exitcode::NOPERM);
    }
    if scan.pids.is_empty() {
        error!("nothing is listening on port {port}");
        process::exit(exitcode::UNAVAILABLE);
    }

    // Re-read the socket table so a PID that stopped holding the port in the
    // meantime is never signalled.
    let still_holding = shut::scan_port(port)
        .map(|scan| scan.pids)
        .unwrap_or_default();
    let me = process::id();
    let mut code = exitcode::OK;
    let mut killed = 0usize;

    for pid in scan.pids {
        if pid == me {
            warn!("refusing to kill shut itself (PID {pid})");
            continue;
        }
        if !still_holding.contains(&pid) {
            warn!("PID {pid} no longer holds port {port}; skipping it");
            continue;
        }

        match shut::kill(pid) {
            Ok(process) => {
                killed += 1;
                if process.command.is_empty() {
                    info!("killed {} (PID {})", process.name, process.pid);
                } else {
                    info!(
                        "killed {} (PID {}) — {}",
                        process.name, process.pid, process.command
                    );
                }
            }
            Err(shut::KillError::Gone) => warn!("PID {pid} exited before it could be killed"),
            Err(shut::KillError::PermissionDenied) => {
                error!("not permitted to kill PID {pid}; retry with sudo");
                code = exitcode::NOPERM;
            }
            Err(shut::KillError::Unsupported(reason)) => {
                error!("cannot kill PID {pid}: {reason}");
                code = exitcode::SOFTWARE;
            }
        }
    }

    if killed == 0 && code == exitcode::OK {
        code = exitcode::UNAVAILABLE;
    }
    process::exit(code);
}

/// Parse the arguments, excluding the program name.
fn parse_args(args: impl Iterator<Item = String>) -> Result<Action, String> {
    let args: Vec<String> = args.collect();

    let Some((first, _)) = args.split_first() else {
        return Err("missing <port>".to_string());
    };

    let action = match first.as_str() {
        "-h" | "--help" => Action::Help,
        "-V" | "--version" => Action::Version,
        raw => Action::Kill(parse_port(raw)?),
    };

    if args.len() > 1 {
        return Err(format!("unexpected argument {:?}", args[1]));
    }
    Ok(action)
}

/// Parse a TCP/UDP port number, rejecting 0 and everything above 65535.
fn parse_port(raw: &str) -> Result<u16, String> {
    match raw.parse::<u16>() {
        Ok(0) => Err("port must be between 1 and 65535".to_string()),
        Ok(port) => Ok(port),
        Err(_) => Err(format!("{raw:?} is not a valid port number")),
    }
}

#[cfg(test)]
mod tests {
    use super::{Action, parse_args, parse_port};

    fn args(list: &[&str]) -> Result<Action, String> {
        parse_args(list.iter().map(ToString::to_string))
    }

    #[test]
    fn parses_ports() {
        assert_eq!(parse_port("1"), Ok(1));
        assert_eq!(parse_port("80"), Ok(80));
        assert_eq!(parse_port("65535"), Ok(65535));
    }

    #[test]
    fn rejects_invalid_ports() {
        for raw in ["0", "65536", "abc", "", "-1", " 80", "80 "] {
            assert!(parse_port(raw).is_err(), "{raw:?} should be rejected");
        }
    }

    #[test]
    fn parses_actions() {
        assert_eq!(args(&["6969"]), Ok(Action::Kill(6969)));
        assert_eq!(args(&["-h"]), Ok(Action::Help));
        assert_eq!(args(&["--help"]), Ok(Action::Help));
        assert_eq!(args(&["-V"]), Ok(Action::Version));
        assert_eq!(args(&["--version"]), Ok(Action::Version));
    }

    #[test]
    fn rejects_missing_and_extra_arguments() {
        assert!(args(&[]).is_err());
        assert!(args(&["6969", "extra"]).is_err());
        assert!(args(&["--help", "extra"]).is_err());
    }
}
