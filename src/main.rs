//! A minimal CLI tool to kill a process listening on a port.
//!
//!  Kill the associated process(es) blocking port 6969:
//! `shut 6969`

use log;
use std::{env, process};

mod shut;

const USAGE: &str = "
Kill process(es) listening on <port>
USAGE:
    shut <port>
";

#[tokio::main]
async fn main() {
    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::builder().format_timestamp(None).init();

    match env::args().nth(1) {
        None => {
            log::info!("{}", USAGE);
            process::exit(exitcode::USAGE);
        }
        Some(arg) => {
            let port: u16 = match arg.parse() {
                Ok(p) => p,
                Err(_) => {
                    log::error!("Port expected: 0..65535");
                    process::exit(exitcode::USAGE);
                }
            };
            log::debug!("Trying to shut port {}", port);
            if shut::test_port(port) {
                log::debug!("Port {} is open", port);

                let pids = shut::get_pids(port);
                if pids.len() == 0 {
                    log::error!("No processes found");
                    process::exit(1);
                }
                log::info!("Process(es) found: {:?}", pids);
                shut::kill(pids).await;
                process::exit(exitcode::OK);
            } else {
                log::info!("Port {} is not open", port);
                process::exit(1);
            }
        }
    };
}
