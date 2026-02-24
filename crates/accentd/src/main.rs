mod compose;
mod grabber;
mod state_machine;
mod uinput_emitter;

use accentd_core::config::{self, Config};
use accentd_core::ipc::{self, ClientMsg, DaemonMsg};
use anyhow::{Context, Result};
use evdev::uinput::VirtualDevice;
use state_machine::{Action, StateMachine};
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{mpsc, Mutex};
use tracing::{error, info, warn};

/// Shared state between the event loop and IPC handlers.
struct Shared {
    config: Config,
    state_machines: Vec<StateMachine>,
    vdev: VirtualDevice,
    /// Channels to send messages to connected popup clients.
    popup_txs: Vec<mpsc::UnboundedSender<String>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("accentd=info".parse().unwrap()),
        )
        .init();

    info!("accentd starting");

    let config = Config::load().context("loading config")?;
    let locale_map = config.load_locale_map().context("loading locale")?;
    info!(locale = %config.locale.active, keys = locale_map.len(), "locale loaded");

    // Find and grab keyboards
    let keyboards = grabber::find_keyboards().context("finding keyboards")?;
    if keyboards.is_empty() {
        anyhow::bail!("no keyboards found — check permissions (group 'input' or udev rules)");
    }

    // Create virtual device
    let vdev = uinput_emitter::create_virtual_device().context("creating virtual device")?;

    // Create per-device state machines
    let state_machines: Vec<StateMachine> = keyboards
        .iter()
        .map(|_| StateMachine::new(&config, locale_map.clone()))
        .collect();

    let shared = Arc::new(Mutex::new(Shared {
        config: config.clone(),
        state_machines,
        vdev,
        popup_txs: Vec::new(),
    }));

    // Event channel from grabbed devices
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();

    // Spawn grabber tasks
    for (idx, path) in keyboards.iter().enumerate() {
        let tx = event_tx.clone();
        let path = path.clone();
        tokio::spawn(async move {
            if let Err(e) = grabber::grab_device(path.clone(), idx, tx).await {
                error!(path = %path.display(), error = %e, "grabber task failed");
            }
        });
    }
    drop(event_tx); // Close our copy so the channel closes when all grabbers exit

    // Start IPC listener
    let socket_path = config::socket_path();
    // Remove stale socket
    let _ = std::fs::remove_file(&socket_path);
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let listener = UnixListener::bind(&socket_path)
        .with_context(|| format!("binding socket {}", socket_path.display()))?;
    // Make socket accessible by the user's session
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o666)).ok();
    }
    info!(path = %socket_path.display(), "IPC socket listening");

    let shared_ipc = Arc::clone(&shared);
    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let shared = Arc::clone(&shared_ipc);
                    tokio::spawn(handle_ipc_client(stream, shared));
                }
                Err(e) => {
                    warn!(error = %e, "IPC accept error");
                }
            }
        }
    });

    // Panic key combo: Backspace → Escape → Enter within 1 second exits the daemon.
    // Safety escape hatch if the daemon hangs with EVIOCGRAB held.
    let mut panic_ring: [(u16, Instant); 3] = [(0, Instant::now()); 3];
    let mut panic_idx: usize = 0;
    const PANIC_SEQ: [u16; 3] = [14, 1, 28]; // KEY_BACKSPACE, KEY_ESC, KEY_ENTER

    // Main event loop: event-driven timer (no idle wakeups)
    loop {
        // Compute the earliest deadline across all state machines
        let deadline = {
            let shared = shared.lock().await;
            shared.state_machines.iter()
                .filter_map(|sm| sm.next_deadline())
                .min()
        };
        let sleep_fut = match deadline {
            Some(dl) => {
                let tokio_instant = tokio::time::Instant::from_std(dl);
                tokio::time::sleep_until(tokio_instant)
            }
            None => tokio::time::sleep_until(tokio::time::Instant::now() + std::time::Duration::from_secs(86400)),
        };
        let has_deadline = deadline.is_some();

        tokio::select! {
            Some(dev_event) = event_rx.recv() => {
                // Check panic key combo (key press events only)
                if dev_event.event.event_type() == evdev::EventType::KEY && dev_event.event.value() == 1 {
                    let code = dev_event.event.code();
                    panic_ring[panic_idx] = (code, Instant::now());
                    panic_idx = (panic_idx + 1) % 3;
                    // Check if the last 3 presses match Backspace→Escape→Enter within 1s
                    let oldest = (panic_idx) % 3;
                    let codes = [
                        panic_ring[oldest].0,
                        panic_ring[(oldest + 1) % 3].0,
                        panic_ring[(oldest + 2) % 3].0,
                    ];
                    if codes == PANIC_SEQ {
                        let elapsed = panic_ring[(oldest + 2) % 3].1.duration_since(panic_ring[oldest].1);
                        if elapsed.as_millis() < 1000 {
                            info!("panic key combo detected (Backspace→Escape→Enter), exiting");
                            std::process::exit(0);
                        }
                    }
                }

                let mut shared = shared.lock().await;
                let idx = dev_event.device_idx;
                if idx < shared.state_machines.len() {
                    let actions = shared.state_machines[idx].process_event(dev_event.event);
                    process_actions(&mut shared, actions);
                }
            }
            _ = sleep_fut, if has_deadline => {
                let mut shared = shared.lock().await;
                let mut all_actions = Vec::new();
                for sm in &mut shared.state_machines {
                    all_actions.extend(sm.check_timer());
                }
                if !all_actions.is_empty() {
                    process_actions(&mut shared, all_actions);
                }
            }
            else => break,
        }
    }

    info!("accentd shutting down");
    let _ = std::fs::remove_file(&socket_path);
    Ok(())
}

fn process_actions(shared: &mut Shared, actions: Vec<Action>) {
    for action in actions {
        match action {
            Action::Relay(event) => {
                if let Err(e) = uinput_emitter::relay_event(&mut shared.vdev, &event) {
                    warn!(error = %e, "relay error");
                }
            }
            Action::SendPopup(msg) => {
                let line = ipc::encode(&msg);
                shared.popup_txs.retain(|tx| tx.send(line.clone()).is_ok());
            }
            Action::EmitAccent(accent) => {
                if let Err(e) = compose::emit_accent(&mut shared.vdev, &accent) {
                    warn!(error = %e, "emit accent error");
                }
            }
            Action::Suppress => {}
        }
    }
}


async fn handle_ipc_client(stream: UnixStream, shared: Arc<Mutex<Shared>>) {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    // Channel for sending messages back to this client
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // Writer task
    let write_handle = tokio::spawn(async move {
        while let Some(line) = rx.recv().await {
            if writer.write_all(line.as_bytes()).await.is_err() {
                break;
            }
        }
    });

    let mut is_popup = false;

    while let Ok(Some(line)) = lines.next_line().await {
        let Some(msg) = ipc::decode_client(&line) else {
            continue;
        };

        let mut shared = shared.lock().await;

        match msg {
            ClientMsg::RegisterPopup => {
                is_popup = true;
                shared.popup_txs.push(tx.clone());
                let ack = DaemonMsg::Ack {
                    ok: true,
                    message: "popup registered".into(),
                };
                let _ = tx.send(ipc::encode(&ack));
            }
            ClientMsg::Select { index } => {
                info!(index, "popup selection via IPC");
                // Find the first SM in Popup state and select
                let actions = shared.state_machines.iter_mut()
                    .find_map(|sm| {
                        let a = sm.ipc_select(index);
                        if a.is_empty() { None } else { Some(a) }
                    })
                    .unwrap_or_default();
                process_actions(&mut shared, actions);
                let ack = DaemonMsg::Ack {
                    ok: true,
                    message: format!("selected {}", index),
                };
                let _ = tx.send(ipc::encode(&ack));
            }
            ClientMsg::Dismiss => {
                info!("popup dismissed via IPC");
                let mut all_actions = Vec::new();
                for sm in &mut shared.state_machines {
                    all_actions.extend(sm.ipc_dismiss());
                }
                process_actions(&mut shared, all_actions);
            }
            ClientMsg::Toggle => {
                let new_state = !shared.state_machines.first().map(|s| s.is_enabled()).unwrap_or(true);
                for sm in &mut shared.state_machines {
                    sm.set_enabled(new_state);
                }
                info!(enabled = new_state, "toggled");
                let ack = DaemonMsg::Ack {
                    ok: true,
                    message: format!("enabled: {}", new_state),
                };
                let _ = tx.send(ipc::encode(&ack));
            }
            ClientMsg::Enable => {
                for sm in &mut shared.state_machines {
                    sm.set_enabled(true);
                }
                let ack = DaemonMsg::Ack {
                    ok: true,
                    message: "enabled".into(),
                };
                let _ = tx.send(ipc::encode(&ack));
            }
            ClientMsg::Disable => {
                for sm in &mut shared.state_machines {
                    sm.set_enabled(false);
                }
                let ack = DaemonMsg::Ack {
                    ok: true,
                    message: "disabled".into(),
                };
                let _ = tx.send(ipc::encode(&ack));
            }
            ClientMsg::SetLocale { locale } => {
                shared.config.locale.active = locale.clone();
                match shared.config.load_locale_map() {
                    Ok(map) => {
                        for sm in &mut shared.state_machines {
                            sm.set_locale_map(map.clone());
                        }
                        let ack = DaemonMsg::Ack {
                            ok: true,
                            message: format!("locale set to {}", locale),
                        };
                        let _ = tx.send(ipc::encode(&ack));
                    }
                    Err(e) => {
                        let ack = DaemonMsg::Ack {
                            ok: false,
                            message: format!("failed to load locale '{}': {}", locale, e),
                        };
                        let _ = tx.send(ipc::encode(&ack));
                    }
                }
            }
            ClientMsg::GetStatus => {
                let enabled = shared.state_machines.first().map(|s| s.is_enabled()).unwrap_or(false);
                let status = DaemonMsg::Status {
                    enabled,
                    locale: shared.config.locale.active.clone(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                };
                let _ = tx.send(ipc::encode(&status));
            }
        }
    }

    // Client disconnected — remove popup sender if registered
    if is_popup {
        let mut shared = shared.lock().await;
        shared.popup_txs.retain(|t| !t.is_closed());
    }

    write_handle.abort();
}
