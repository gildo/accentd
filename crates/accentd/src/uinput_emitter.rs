use crate::compose::EventEmitter;
use anyhow::{Context, Result};
use evdev::uinput::VirtualDeviceBuilder;
use evdev::uinput::VirtualDevice;
use evdev::{AttributeSet, InputEvent, Key};
use tracing::info;

pub fn create_virtual_device() -> Result<VirtualDevice> {
    let mut keys = AttributeSet::<Key>::new();
    for code in 0..=255u16 {
        keys.insert(Key::new(code));
    }

    let vdev = VirtualDeviceBuilder::new()
        .context("creating VirtualDeviceBuilder")?
        .name("accentd virtual keyboard")
        .with_keys(&keys)
        .context("setting keys")?
        .build()
        .context("building virtual device")?;

    info!("virtual uinput device created");
    Ok(vdev)
}

pub fn relay_event(emitter: &mut impl EventEmitter, event: &InputEvent) -> Result<()> {
    emitter.emit_events(&[*event])?;
    Ok(())
}
