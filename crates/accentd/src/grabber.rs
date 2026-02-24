use anyhow::{Context, Result};
use evdev::{Device, InputEvent};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

/// An event from a grabbed keyboard, tagged with the device index.
#[derive(Debug, Clone)]
pub struct DeviceEvent {
    pub device_idx: usize,
    pub event: InputEvent,
}

/// Find all keyboard devices under /dev/input/.
pub fn find_keyboards() -> Result<Vec<PathBuf>> {
    let mut keyboards = Vec::new();
    let input_dir = Path::new("/dev/input");

    for entry in std::fs::read_dir(input_dir).context("reading /dev/input")? {
        let entry = entry?;
        let path = entry.path();

        // Only look at eventN devices
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !name.starts_with("event") {
            continue;
        }

        match Device::open(&path) {
            Ok(dev) => {
                // Skip our own virtual device to avoid feedback loop
                if dev.name().map_or(false, |n| n.contains("accentd")) {
                    debug!(path = %path.display(), name = ?dev.name(), "skipping own virtual device");
                    continue;
                }
                if is_keyboard(&dev) {
                    info!(path = %path.display(), name = ?dev.name(), "found keyboard");
                    keyboards.push(path);
                }
            }
            Err(e) => {
                debug!(path = %path.display(), error = %e, "skipping device");
            }
        }
    }

    Ok(keyboards)
}

/// Heuristic: a device is a keyboard if it has KEY events and supports
/// common letter keys (KEY_A through KEY_Z).
fn is_keyboard(dev: &Device) -> bool {
    let Some(keys) = dev.supported_keys() else {
        return false;
    };

    // Must have KEY_A (30), KEY_Z (44), KEY_ENTER (28)
    keys.contains(evdev::Key::KEY_A)
        && keys.contains(evdev::Key::KEY_Z)
        && keys.contains(evdev::Key::KEY_ENTER)
}

/// Grab a keyboard device and forward events to the channel.
/// Runs until the sender is dropped or the device errors.
pub async fn grab_device(
    path: PathBuf,
    device_idx: usize,
    tx: mpsc::UnboundedSender<DeviceEvent>,
) -> Result<()> {
    let mut dev = Device::open(&path)
        .with_context(|| format!("opening {}", path.display()))?;

    let dev_name = dev.name().unwrap_or("unknown").to_string();
    info!(device = %dev_name, path = %path.display(), "grabbing device");

    dev.grab()
        .with_context(|| format!("grabbing {}", path.display()))?;

    // Wrap in tokio AsyncDevice for non-blocking reads
    let mut stream = dev.into_event_stream()
        .context("creating event stream")?;

    loop {
        match stream.next_event().await {
            Ok(event) => {
                if tx.send(DeviceEvent { device_idx, event }).is_err() {
                    // Receiver dropped, shut down
                    break;
                }
            }
            Err(e) => {
                warn!(device = %dev_name, error = %e, "device error, stopping grab");
                break;
            }
        }
    }

    Ok(())
}
