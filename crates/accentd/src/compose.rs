use anyhow::Result;
use evdev::uinput::VirtualDevice;
use evdev::{EventType, InputEvent, Key};
use std::time::Duration;

fn tap_key(vdev: &mut VirtualDevice, key: Key) -> Result<()> {
    let press = InputEvent::new(EventType::KEY, key.code(), 1);
    let release = InputEvent::new(EventType::KEY, key.code(), 0);
    let syn = InputEvent::new(EventType::SYNCHRONIZATION, 0, 0);
    vdev.emit(&[press, syn])?;
    std::thread::sleep(Duration::from_millis(2));
    vdev.emit(&[release, syn])?;
    Ok(())
}

fn hold_key(vdev: &mut VirtualDevice, key: Key, press: bool) -> Result<()> {
    let val = if press { 1 } else { 0 };
    let ev = InputEvent::new(EventType::KEY, key.code(), val);
    let syn = InputEvent::new(EventType::SYNCHRONIZATION, 0, 0);
    vdev.emit(&[ev, syn])?;
    std::thread::sleep(Duration::from_millis(2));
    Ok(())
}

/// Emit a backspace to delete the base character, then emit the accented character
/// via Ctrl+Shift+U hex sequence (GTK/Qt Unicode input method).
/// NOTE: Ctrl+Shift+U works in GTK and Qt apps. It may fail in Electron apps,
/// some terminal emulators, and other toolkits that don't support this input method.
pub fn emit_accent(vdev: &mut VirtualDevice, accent: &str) -> Result<()> {
    // Wait for popup hide to be processed
    std::thread::sleep(Duration::from_millis(20));
    tap_key(vdev, Key::KEY_BACKSPACE)?;

    let c = accent.chars().next().unwrap_or(' ');
    let hex = format!("{:04x}", c as u32);

    // Modifiers need time to register before the U key
    hold_key(vdev, Key::KEY_LEFTCTRL, true)?;
    hold_key(vdev, Key::KEY_LEFTSHIFT, true)?;
    tap_key(vdev, Key::KEY_U)?;
    hold_key(vdev, Key::KEY_LEFTSHIFT, false)?;
    hold_key(vdev, Key::KEY_LEFTCTRL, false)?;

    for digit in hex.chars() {
        tap_key(vdev, hex_char_to_key(digit))?;
    }

    tap_key(vdev, Key::KEY_ENTER)?;
    Ok(())
}

fn hex_char_to_key(c: char) -> Key {
    match c {
        '0' => Key::KEY_0,
        '1' => Key::KEY_1,
        '2' => Key::KEY_2,
        '3' => Key::KEY_3,
        '4' => Key::KEY_4,
        '5' => Key::KEY_5,
        '6' => Key::KEY_6,
        '7' => Key::KEY_7,
        '8' => Key::KEY_8,
        '9' => Key::KEY_9,
        'a' => Key::KEY_A,
        'b' => Key::KEY_B,
        'c' => Key::KEY_C,
        'd' => Key::KEY_D,
        'e' => Key::KEY_E,
        'f' => Key::KEY_F,
        _ => Key::KEY_0,
    }
}
