use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread::sleep;
use std::time::{Duration, Instant};

use shut::{KillError, scan_port};

static NEXT_ID: AtomicU32 = AtomicU32::new(0);

/// Child half of [`spawn_listener`].
///
/// Spawned as this test binary with `SHUT_TEST_LISTEN` set to a file path:
/// binds an ephemeral loopback port, writes the port number to that file, then
/// blocks until it is killed. No-ops when the variable is unset, so a plain
/// `cargo test` run ignores it.
#[test]
fn child_listener() {
    let Ok(path) = std::env::var("SHUT_TEST_LISTEN") else {
        return;
    };
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let port = listener.local_addr().expect("addr").port();
    std::fs::write(path, port.to_string()).expect("announce port");
    for stream in listener.incoming() {
        drop(stream);
    }
}

/// Start a child that listens on an ephemeral loopback port.
fn spawn_listener() -> (Child, u16) {
    let path = std::env::temp_dir().join(format!(
        "shut-test-{}-{}",
        std::process::id(),
        NEXT_ID.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_file(&path);

    let mut child = Command::new(std::env::current_exe().expect("current exe"))
        .args(["--exact", "child_listener"])
        .env("SHUT_TEST_LISTEN", &path)
        .stdout(Stdio::null())
        .spawn()
        .expect("spawn listener");

    match read_announced_port(&path) {
        Some(port) => (child, port),
        None => {
            let _ = child.kill();
            let _ = child.wait();
            panic!("child never announced a port via {}", path.display());
        }
    }
}

/// Wait for the child to publish its port in `path`.
fn read_announced_port(path: &std::path::Path) -> Option<u16> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(text) = std::fs::read_to_string(path)
            && let Ok(port) = text.trim().parse::<u16>()
        {
            let _ = std::fs::remove_file(path);
            return Some(port);
        }
        if Instant::now() >= deadline {
            return None;
        }
        sleep(Duration::from_millis(20));
    }
}

/// Poll until the socket table attributes `port` to a process.
fn wait_until_visible(port: u16) -> shut::PortScan {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let scan = scan_port(port).expect("scan");
        if !scan.pids.is_empty() {
            return scan;
        }
        assert!(
            Instant::now() < deadline,
            "port {port} never became visible"
        );
        sleep(Duration::from_millis(20));
    }
}

#[test]
fn kills_a_listening_process() {
    let (mut child, port) = spawn_listener();
    let pid = child.id();

    let scan = wait_until_visible(port);
    assert_eq!(scan.pids, vec![pid], "the child should own the port");

    let killed = shut::kill(pid).expect("kill listener");
    assert_eq!(killed.pid, pid);
    assert!(!killed.name.is_empty());

    child.wait().expect("reap child");
    assert!(scan_port(port).expect("rescan").pids.is_empty());
}

#[test]
fn reports_gone_for_a_finished_process() {
    let (mut child, port) = spawn_listener();
    let pid = child.id();
    wait_until_visible(port);

    shut::kill(pid).expect("kill listener");
    child.wait().expect("reap child");

    assert_eq!(shut::kill(pid), Err(KillError::Gone));
}

#[test]
fn reports_nothing_bound_to_a_free_port() {
    let port = {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
        listener.local_addr().expect("addr").port()
    };

    let scan = scan_port(port).expect("scan");
    assert!(scan.pids.is_empty());
    assert!(!scan.owner_is_hidden());
}
