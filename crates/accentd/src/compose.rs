use anyhow::Result;
use evdev::uinput::VirtualDevice;
use evdev::{EventType, InputEvent, Key};
use std::time::Duration;

pub trait EventEmitter {
    fn emit_events(&mut self, events: &[InputEvent]) -> Result<()>;
}

impl EventEmitter for VirtualDevice {
    fn emit_events(&mut self, events: &[InputEvent]) -> Result<()> {
        self.emit(events)?;
        Ok(())
    }
}

/// Wait for the popup window to hide and focus to return to the target window.
const DELAY_POPUP_HIDE: Duration = Duration::from_millis(50);

/// After backspace, give the app time to process the deletion before we start
/// the Ctrl+Shift+U sequence.
const DELAY_AFTER_BACKSPACE: Duration = Duration::from_millis(5);

/// Minimum delay between individual evdev emits. Modifier state changes and
/// key press/release pairs each need a kernel round-trip to register properly.
const DELAY_BETWEEN_EMITS: Duration = Duration::from_millis(3);

/// After the full Ctrl+Shift+U chord and modifier release, the app needs a
/// moment to enter Unicode hex input mode before it can accept hex digits.
const DELAY_AFTER_CHORD: Duration = Duration::from_millis(5);

fn syn() -> InputEvent {
    InputEvent::new(EventType::SYNCHRONIZATION, 0, 0)
}

/// Tap a key: press, sleep, release, sleep. Each half needs its own emit so the
/// kernel processes the state change before the next event.
fn tap_key(emitter: &mut impl EventEmitter, key: Key) -> Result<()> {
    emitter.emit_events(&[InputEvent::new(EventType::KEY, key.code(), 1), syn()])?;
    std::thread::sleep(DELAY_BETWEEN_EMITS);
    emitter.emit_events(&[InputEvent::new(EventType::KEY, key.code(), 0), syn()])?;
    std::thread::sleep(DELAY_BETWEEN_EMITS);
    Ok(())
}

/// Press or release a single key + syn, then sleep.
fn hold_key(emitter: &mut impl EventEmitter, key: Key, press: bool) -> Result<()> {
    let val = if press { 1 } else { 0 };
    emitter.emit_events(&[InputEvent::new(EventType::KEY, key.code(), val), syn()])?;
    std::thread::sleep(DELAY_BETWEEN_EMITS);
    Ok(())
}

/// Emit a backspace to delete the base character, then emit the accented character
/// via Ctrl+Shift+U hex sequence (GTK/Qt Unicode input method).
///
/// The protocol has 4 phases:
///   1. Backspace — delete the base character
///   2. Ctrl+Shift+U chord — enter Unicode hex input mode
///   3. Hex digits + Enter — type the codepoint and confirm
///
/// NOTE: Ctrl+Shift+U works in GTK and Qt apps. It may fail in Electron apps,
/// some terminal emulators, and other toolkits that don't support this input method.
pub fn emit_accent(emitter: &mut impl EventEmitter, accent: &str) -> Result<()> {
    let c = accent.chars().next().unwrap_or(' ');
    let hex = format!("{:04x}", c as u32);

    // Wait for popup to hide and focus to return
    std::thread::sleep(DELAY_POPUP_HIDE);

    // Phase 1: delete the base character
    tap_key(emitter, Key::KEY_BACKSPACE)?;
    std::thread::sleep(DELAY_AFTER_BACKSPACE);

    // Phase 2: Ctrl+Shift+U chord — each modifier and the U tap need separate
    // emits so the kernel registers the state changes in order
    hold_key(emitter, Key::KEY_LEFTCTRL, true)?;
    hold_key(emitter, Key::KEY_LEFTSHIFT, true)?;
    tap_key(emitter, Key::KEY_U)?;
    hold_key(emitter, Key::KEY_LEFTSHIFT, false)?;
    hold_key(emitter, Key::KEY_LEFTCTRL, false)?;
    std::thread::sleep(DELAY_AFTER_CHORD);

    // Phase 3: hex digits + Enter, batched in a single emit (no inter-event
    // delays needed — Unicode input mode is already active)
    let mut events = Vec::with_capacity((hex.len() + 1) * 4);
    for digit in hex.chars() {
        let key = hex_char_to_key(digit);
        events.push(InputEvent::new(EventType::KEY, key.code(), 1));
        events.push(syn());
        events.push(InputEvent::new(EventType::KEY, key.code(), 0));
        events.push(syn());
    }
    events.push(InputEvent::new(EventType::KEY, Key::KEY_ENTER.code(), 1));
    events.push(syn());
    events.push(InputEvent::new(EventType::KEY, Key::KEY_ENTER.code(), 0));
    events.push(syn());
    emitter.emit_events(&events)?;

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

#[cfg(test)]
mod tests {
    use super::*;

    struct RecordingEmitter {
        batches: Vec<Vec<InputEvent>>,
    }

    impl RecordingEmitter {
        fn new() -> Self {
            Self { batches: Vec::new() }
        }

        /// Extract (key_code, value) pairs from a batch, filtering out SYN events.
        fn key_events(&self, batch_idx: usize) -> Vec<(u16, i32)> {
            self.batches[batch_idx]
                .iter()
                .filter(|e| e.event_type() == EventType::KEY)
                .map(|e| (e.code(), e.value()))
                .collect()
        }

        /// Flatten all batches into (key_code, value) pairs, filtering out SYN.
        fn all_key_events(&self) -> Vec<(u16, i32)> {
            self.batches
                .iter()
                .flat_map(|batch| batch.iter())
                .filter(|e| e.event_type() == EventType::KEY)
                .map(|e| (e.code(), e.value()))
                .collect()
        }
    }

    impl EventEmitter for RecordingEmitter {
        fn emit_events(&mut self, events: &[InputEvent]) -> Result<()> {
            self.batches.push(events.to_vec());
            Ok(())
        }
    }

    #[test]
    fn modifier_presses_are_separate_emits() {
        let mut mock = RecordingEmitter::new();
        emit_accent(&mut mock, "è").unwrap();
        // Batch 2 = Ctrl↓, batch 3 = Shift↓ (after BS↓, BS↑)
        assert_eq!(mock.key_events(2), vec![(Key::KEY_LEFTCTRL.code(), 1)]);
        assert_eq!(mock.key_events(3), vec![(Key::KEY_LEFTSHIFT.code(), 1)]);
    }

    #[test]
    fn modifier_releases_are_separate_emits() {
        let mut mock = RecordingEmitter::new();
        emit_accent(&mut mock, "è").unwrap();
        // After U tap (batches 4,5), Shift↑ = batch 6, Ctrl↑ = batch 7
        assert_eq!(mock.key_events(6), vec![(Key::KEY_LEFTSHIFT.code(), 0)]);
        assert_eq!(mock.key_events(7), vec![(Key::KEY_LEFTCTRL.code(), 0)]);
    }

    #[test]
    fn key_tap_is_two_emits() {
        let mut mock = RecordingEmitter::new();
        emit_accent(&mut mock, "è").unwrap();
        // Backspace tap = batches 0 (press) and 1 (release)
        assert_eq!(mock.key_events(0), vec![(Key::KEY_BACKSPACE.code(), 1)]);
        assert_eq!(mock.key_events(1), vec![(Key::KEY_BACKSPACE.code(), 0)]);
        // U tap = batches 4 (press) and 5 (release)
        assert_eq!(mock.key_events(4), vec![(Key::KEY_U.code(), 1)]);
        assert_eq!(mock.key_events(5), vec![(Key::KEY_U.code(), 0)]);
    }

    #[test]
    fn hex_digits_are_one_emit() {
        let mut mock = RecordingEmitter::new();
        emit_accent(&mut mock, "è").unwrap();
        // Last batch (8) contains all hex digit press/release pairs + Enter
        let last = mock.batches.len() - 1;
        let key_events = mock.key_events(last);
        // è = U+00E8 → 4 hex digits + Enter = 5 key taps = 10 key events
        assert_eq!(key_events.len(), 10);
    }

    #[test]
    fn full_event_sequence() {
        let mut mock = RecordingEmitter::new();
        emit_accent(&mut mock, "è").unwrap();
        let events = mock.all_key_events();
        let expected = vec![
            // BS tap
            (Key::KEY_BACKSPACE.code(), 1),
            (Key::KEY_BACKSPACE.code(), 0),
            // Ctrl+Shift+U chord
            (Key::KEY_LEFTCTRL.code(), 1),
            (Key::KEY_LEFTSHIFT.code(), 1),
            (Key::KEY_U.code(), 1),
            (Key::KEY_U.code(), 0),
            (Key::KEY_LEFTSHIFT.code(), 0),
            (Key::KEY_LEFTCTRL.code(), 0),
            // hex 00e8 + Enter
            (Key::KEY_0.code(), 1),
            (Key::KEY_0.code(), 0),
            (Key::KEY_0.code(), 1),
            (Key::KEY_0.code(), 0),
            (Key::KEY_E.code(), 1),
            (Key::KEY_E.code(), 0),
            (Key::KEY_8.code(), 1),
            (Key::KEY_8.code(), 0),
            (Key::KEY_ENTER.code(), 1),
            (Key::KEY_ENTER.code(), 0),
        ];
        assert_eq!(events, expected);
    }

    #[test]
    fn emit_accent_for_e_grave() {
        let mut mock = RecordingEmitter::new();
        emit_accent(&mut mock, "è").unwrap();
        // è = U+00E8 → hex digits in last batch should be KEY_0, KEY_0, KEY_E, KEY_8
        let last = mock.batches.len() - 1;
        let hex_keys: Vec<u16> = mock.key_events(last)
            .iter()
            .filter(|(_, v)| *v == 1) // just presses
            .map(|(c, _)| *c)
            .collect();
        // 4 hex digit presses + Enter press
        assert_eq!(
            hex_keys,
            vec![
                Key::KEY_0.code(),
                Key::KEY_0.code(),
                Key::KEY_E.code(),
                Key::KEY_8.code(),
                Key::KEY_ENTER.code(),
            ]
        );
    }

    #[test]
    fn hex_char_to_key_maps_all_hex_digits() {
        assert_eq!(hex_char_to_key('0'), Key::KEY_0);
        assert_eq!(hex_char_to_key('1'), Key::KEY_1);
        assert_eq!(hex_char_to_key('2'), Key::KEY_2);
        assert_eq!(hex_char_to_key('3'), Key::KEY_3);
        assert_eq!(hex_char_to_key('4'), Key::KEY_4);
        assert_eq!(hex_char_to_key('5'), Key::KEY_5);
        assert_eq!(hex_char_to_key('6'), Key::KEY_6);
        assert_eq!(hex_char_to_key('7'), Key::KEY_7);
        assert_eq!(hex_char_to_key('8'), Key::KEY_8);
        assert_eq!(hex_char_to_key('9'), Key::KEY_9);
        assert_eq!(hex_char_to_key('a'), Key::KEY_A);
        assert_eq!(hex_char_to_key('b'), Key::KEY_B);
        assert_eq!(hex_char_to_key('c'), Key::KEY_C);
        assert_eq!(hex_char_to_key('d'), Key::KEY_D);
        assert_eq!(hex_char_to_key('e'), Key::KEY_E);
        assert_eq!(hex_char_to_key('f'), Key::KEY_F);
    }
}
