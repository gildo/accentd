use accentd_core::config;
use accentd_core::ipc::{self, ClientMsg, DaemonMsg};
use anyhow::{Context, Result};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::sync::mpsc as std_mpsc;
use tracing::{info, warn};

fn setup_connection(stream: UnixStream) -> Result<(std_mpsc::Receiver<DaemonMsg>, UnixStream)> {
    let mut write_stream = stream.try_clone().context("cloning stream")?;

    let register = ipc::encode(&ClientMsg::RegisterPopup);
    write_stream
        .write_all(register.as_bytes())
        .context("sending register")?;

    let (tx, rx) = std_mpsc::channel();

    let read_stream = stream;
    std::thread::spawn(move || {
        let reader = BufReader::new(read_stream);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    if let Some(msg) = ipc::decode_daemon(&line) {
                        if tx.send(msg).is_err() {
                            break;
                        }
                    }
                }
                Err(e) => {
                    warn!(error = %e, "IPC read error");
                    break;
                }
            }
        }
        info!("IPC reader thread exiting");
    });

    Ok((rx, write_stream))
}

/// Connect to the daemon with retries (for initial startup).
pub fn connect() -> Result<(std_mpsc::Receiver<DaemonMsg>, UnixStream)> {
    let socket_path = config::socket_path();
    let stream = {
        let mut result = UnixStream::connect(&socket_path);
        for _ in 0..9 {
            if result.is_ok() { break; }
            std::thread::sleep(std::time::Duration::from_millis(500));
            result = UnixStream::connect(&socket_path);
        }
        result.with_context(|| format!("connecting to {}", socket_path.display()))?
    };
    setup_connection(stream)
}

/// Try to connect once without retries (for reconnection).
pub fn try_connect() -> Result<(std_mpsc::Receiver<DaemonMsg>, UnixStream)> {
    let socket_path = config::socket_path();
    let stream = UnixStream::connect(&socket_path)
        .with_context(|| format!("connecting to {}", socket_path.display()))?;
    setup_connection(stream)
}
