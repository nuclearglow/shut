//! Find and kill the processes holding a TCP or UDP port.
//!
//! The port is resolved from the kernel's socket table, so IPv4-only,
//! IPv6-only, dual-stack and UDP listeners are all found. Detection never
//! performs a TCP connect: connecting to a port only proves that *something*
//! answers on one address family, which misses IPv6-only, UDP and
//! non-loopback binds.

use std::collections::BTreeSet;
use std::io;

use netstat2::{AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, TcpState, get_sockets_info};
use sysinfo::{Pid, ProcessesToUpdate, Signal, System};

/// Sockets bound to a port, and the processes owning them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PortScan {
    /// Owning PIDs, deduplicated and sorted.
    ///
    /// Empty when nothing is bound to the port, or when the owning processes
    /// belong to another user and their file descriptors are unreadable.
    pub pids: Vec<u32>,
    /// Number of sockets bound to the port, over all address families.
    ///
    /// A TCP socket only counts while it is listening; an established
    /// connection whose local port happens to match is not a listener.
    pub sockets: usize,
}

impl PortScan {
    /// `true` when a socket is bound but its owner is not visible to us.
    ///
    /// Reported as a permission problem rather than as "nothing listens here".
    #[must_use]
    pub fn owner_is_hidden(&self) -> bool {
        self.sockets > 0 && self.pids.is_empty()
    }
}

/// A process that was signalled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Killed {
    pub pid: u32,
    pub name: String,
    pub command: String,
}

/// Why a signal could not be delivered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KillError {
    /// The process no longer exists.
    Gone,
    /// The process belongs to another user.
    PermissionDenied,
    /// Platform surface we cannot interpret.
    Unsupported(String),
}

/// Look up every socket bound to `port`.
///
/// # Errors
///
/// Fails when the kernel socket table cannot be read.
pub fn scan_port(port: u16) -> io::Result<PortScan> {
    let sockets = get_sockets_info(
        AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6,
        ProtocolFlags::TCP | ProtocolFlags::UDP,
    )
    .map_err(|error| io::Error::other(format!("failed to read the socket table: {error}")))?;

    let mut pids = BTreeSet::new();
    let mut bound = 0usize;

    for socket in sockets {
        let holds_port = match &socket.protocol_socket_info {
            ProtocolSocketInfo::Tcp(tcp) => tcp.local_port == port && tcp.state == TcpState::Listen,
            ProtocolSocketInfo::Udp(udp) => udp.local_port == port,
        };
        if !holds_port {
            continue;
        }
        bound += 1;
        pids.extend(socket.associated_pids);
    }

    Ok(PortScan {
        pids: pids.into_iter().collect(),
        sockets: bound,
    })
}

/// Send `SIGKILL` to `pid`.
///
/// The process is looked up first, so the returned description reflects what
/// was signalled. A signal that is not delivered is classified by re-checking
/// whether the process is still alive.
///
/// # Errors
///
/// Returns a [`KillError`] describing why the process was not signalled.
pub fn kill(pid: u32) -> Result<Killed, KillError> {
    let mut system = System::new().map_err(|error| {
        KillError::Unsupported(format!("cannot read the process table: {error}"))
    })?;
    let pids = [Pid::from_u32(pid)];

    system.refresh_processes(ProcessesToUpdate::Some(&pids), true);
    let Some(process) = system.process(pids[0]) else {
        return Err(KillError::Gone);
    };

    let killed = Killed {
        pid,
        name: process.name().to_string_lossy().into_owned(),
        command: process
            .cmd()
            .iter()
            .map(|arg| arg.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" "),
    };

    if process.kill_with(Signal::Kill) == Some(true) {
        return Ok(killed);
    }

    system.refresh_processes(ProcessesToUpdate::Some(&pids), true);
    if system.process(pids[0]).is_some() {
        Err(KillError::PermissionDenied)
    } else {
        Err(KillError::Gone)
    }
}
