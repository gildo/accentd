use accentd_core::config;
use accentd_core::ipc::{self, ClientMsg, DaemonMsg};
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

#[derive(Parser)]
#[command(name = "accentctl", about = "Control the accentd daemon")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show daemon status
    Status,
    /// Enable accent detection
    Enable,
    /// Disable accent detection
    Disable,
    /// Toggle accent detection on/off
    Toggle,
    /// Set the active locale
    SetLocale {
        /// Locale name (e.g., it, es, fr, de, pt)
        locale: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let socket_path = config::socket_path();
    let stream = UnixStream::connect(&socket_path)
        .with_context(|| format!("connecting to accentd at {}\nIs the daemon running?", socket_path.display()))?;

    let mut writer = stream.try_clone().context("cloning stream")?;
    let reader = BufReader::new(stream);

    let msg: ClientMsg = match cli.command {
        Command::Status => ClientMsg::GetStatus,
        Command::Enable => ClientMsg::Enable,
        Command::Disable => ClientMsg::Disable,
        Command::Toggle => ClientMsg::Toggle,
        Command::SetLocale { locale } => ClientMsg::SetLocale { locale },
    };

    let line = ipc::encode(&msg);
    writer
        .write_all(line.as_bytes())
        .context("sending command")?;

    // Read response
    for line in reader.lines() {
        let line = line.context("reading response")?;
        if let Some(resp) = ipc::decode_daemon(&line) {
            match resp {
                DaemonMsg::Status {
                    enabled,
                    locale,
                    version,
                } => {
                    println!("accentd v{}", version);
                    println!("  enabled: {}", enabled);
                    println!("  locale:  {}", locale);
                }
                DaemonMsg::Ack { ok, message } => {
                    if ok {
                        println!("{}", message);
                    } else {
                        eprintln!("error: {}", message);
                        std::process::exit(1);
                    }
                }
                _ => {}
            }
            break;
        }
    }

    Ok(())
}
