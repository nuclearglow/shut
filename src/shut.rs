use heim::process::processes;
use log::{error, info};
use netstat2::{get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo};
use std::net::TcpStream;
use tokio_stream::StreamExt;

/// Scan a port and return true if it's open.
pub fn test_port(port: u16) -> bool {
    match TcpStream::connect(("0.0.0.0", port)) {
        Ok(_) => true,
        Err(_) => false,
    }
}

/// Get a port and return the associated process ID(s)
pub fn get_pids(port: u16) -> Vec<u32> {
    let af_flags = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
    let proto_flags = ProtocolFlags::TCP | ProtocolFlags::UDP;
    let sockets_info = get_sockets_info(af_flags, proto_flags).unwrap_or_default();

    let process =
        sockets_info
            .into_iter()
            .find(|socket_info| match &socket_info.protocol_socket_info {
                ProtocolSocketInfo::Tcp(tcp) => tcp.local_port == port,
                ProtocolSocketInfo::Udp(udp) => udp.local_port == port,
            });

    match process {
        Some(socket_info) => socket_info.associated_pids,
        None => Vec::with_capacity(0),
    }
}

/// Match all processes by given pids and try to kill them
pub async fn kill(pids: Vec<u32>) {
    let all_processes = match processes().await {
        Ok(processes) => processes.filter_map(|process| process.ok()).collect().await,
        Err(_) => Vec::with_capacity(0),
    };

    for process in all_processes {
        let pid = process.pid() as u32;
        let selected_process = pids.contains(&pid);

        if selected_process {
            let command = match process.command().await {
                Ok(c) => format!("{:?}", c),
                Err(_) => String::from(""),
            };
            let process_info = match process.name().await {
                Ok(name) => format!("{:?} (PID: {})", name, &pid),
                Err(_) => format!("{:?}", &pid),
            };
            info!("Killed {} {}", process_info, command);
            if let Err(error) = process.kill().await {
                error!("Error: {}", error.to_string());
            }
        }
    }
}
