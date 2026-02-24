use accentd_core::charmap;
use accentd_core::config::Config;
use accentd_core::ipc::DaemonMsg;
use evdev::{EventType, InputEvent, Key};
use std::collections::HashMap;
use std::time::Instant;
use tracing::debug;

/// Per-device state machine states.
#[derive(Debug, Clone, PartialEq)]
enum State {
    /// Normal passthrough mode.
    Idle,
    /// An accent-eligible key was pressed; waiting for threshold.
    Holding {
        base: String,
        accents: Vec<String>,
        key_code: u16,
        shift: bool,
        started: Instant,
    },
    /// Popup is shown, awaiting number selection or dismiss.
    Popup {
        base: String,
        accents: Vec<String>,
        key_code: u16,
        started: Instant,
    },
}

/// Actions that the state machine wants the caller to perform.
#[derive(Debug)]
pub enum Action {
    /// Relay event to uinput unchanged.
    Relay(InputEvent),
    /// Send a message to the popup UI.
    SendPopup(DaemonMsg),
    /// Emit an accented character (backspace + char).
    EmitAccent(String),
    /// Suppress this event (don't relay).
    Suppress,
}

pub struct StateMachine {
    state: State,
    locale_map: HashMap<String, Vec<String>>,
    threshold_ms: u64,
    popup_timeout_ms: u64,
    enabled: bool,
    /// Track modifier state.
    ctrl_held: bool,
    alt_held: bool,
    super_held: bool,
    shift_held: bool,
}

impl StateMachine {
    pub fn new(config: &Config, locale_map: HashMap<String, Vec<String>>) -> Self {
        Self {
            state: State::Idle,
            locale_map,
            threshold_ms: config.general.threshold_ms,
            popup_timeout_ms: config.popup.timeout_ms,
            enabled: config.general.enabled,
            ctrl_held: false,
            alt_held: false,
            super_held: false,
            shift_held: false,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.state = State::Idle;
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_locale_map(&mut self, map: HashMap<String, Vec<String>>) {
        self.locale_map = map;
        self.state = State::Idle;
    }

    /// Check if we're in HOLDING state and the threshold has elapsed,
    /// or in Popup state and the timeout has elapsed.
    pub fn check_timer(&mut self) -> Vec<Action> {
        match &self.state {
            State::Holding {
                ref base,
                ref accents,
                key_code,
                started,
                ..
            } => {
                if started.elapsed().as_millis() as u64 >= self.threshold_ms {
                    debug!(base = %base, "hold threshold reached, showing popup");

                    let release = InputEvent::new(EventType::KEY, *key_code, 0);
                    let labels: Vec<u8> = (1..=accents.len() as u8).collect();
                    let actions = vec![
                        Action::Relay(release),
                        Action::SendPopup(DaemonMsg::ShowPopup {
                            base: base.clone(),
                            accents: accents.clone(),
                            labels,
                        }),
                    ];
                    self.state = State::Popup {
                        base: base.clone(),
                        accents: accents.clone(),
                        key_code: *key_code,
                        started: Instant::now(),
                    };
                    actions
                } else {
                    Vec::new()
                }
            }
            State::Popup { started, .. } => {
                if started.elapsed().as_millis() as u64 >= self.popup_timeout_ms {
                    debug!("popup timed out");
                    self.state = State::Idle;
                    vec![Action::SendPopup(DaemonMsg::HidePopup)]
                } else {
                    Vec::new()
                }
            }
            State::Idle => Vec::new(),
        }
    }

    /// Return the next `Instant` at which `check_timer()` needs to run,
    /// or `None` if idle (no timer needed).
    pub fn next_deadline(&self) -> Option<Instant> {
        match &self.state {
            State::Holding { started, .. } => {
                Some(*started + std::time::Duration::from_millis(self.threshold_ms))
            }
            State::Popup { started, .. } => {
                Some(*started + std::time::Duration::from_millis(self.popup_timeout_ms))
            }
            State::Idle => None,
        }
    }

    /// IPC: select accent by 1-indexed number. Returns actions if in Popup state.
    pub fn ipc_select(&mut self, index: u8) -> Vec<Action> {
        if let State::Popup { ref accents, .. } = self.state {
            let idx = (index - 1) as usize;
            if idx < accents.len() {
                let accent = accents[idx].clone();
                self.state = State::Idle;
                return vec![
                    Action::SendPopup(DaemonMsg::HidePopup),
                    Action::EmitAccent(accent),
                ];
            }
        }
        Vec::new()
    }

    /// IPC: dismiss popup. Returns actions if in Popup state.
    pub fn ipc_dismiss(&mut self) -> Vec<Action> {
        if matches!(self.state, State::Popup { .. }) {
            self.state = State::Idle;
            return vec![Action::SendPopup(DaemonMsg::HidePopup)];
        }
        Vec::new()
    }

    /// Process an input event, returning actions for the caller.
    pub fn process_event(&mut self, event: InputEvent) -> Vec<Action> {
        // Track modifier state for all events
        if event.event_type() == EventType::KEY {
            self.update_modifiers(&event);
        }

        // Non-key events: always relay
        if event.event_type() != EventType::KEY {
            return vec![Action::Relay(event)];
        }

        let code = event.code();
        let value = event.value(); // 0=release, 1=press, 2=repeat

        if !self.enabled {
            return vec![Action::Relay(event)];
        }

        match &self.state {
            State::Idle => self.handle_idle(event, code, value),
            State::Holding { .. } => self.handle_holding(event, code, value),
            State::Popup { .. } => self.handle_popup(event, code, value),
        }
    }

    fn handle_idle(&mut self, event: InputEvent, code: u16, value: i32) -> Vec<Action> {
        // Only interested in key press (value=1)
        if value != 1 {
            return vec![Action::Relay(event)];
        }

        // If a modifier is held, don't start accent detection
        if self.ctrl_held || self.alt_held || self.super_held {
            return vec![Action::Relay(event)];
        }

        // Check if this is an accent-eligible key
        let shift = self.shift_held;
        if let Some(base) = charmap::keycode_to_base(code) {
            if let Some(accents) = charmap::resolve_accents(&self.locale_map, base, shift) {
                if !accents.is_empty() {
                    debug!(base = %base, shift, "starting hold timer");
                    self.state = State::Holding {
                        base: base.to_string(),
                        accents,
                        key_code: code,
                        shift,
                        started: Instant::now(),
                    };
                    // Emit the base key immediately (zero latency)
                    return vec![Action::Relay(event)];
                }
            }
        }

        // Not accent-eligible, relay normally
        vec![Action::Relay(event)]
    }

    fn handle_holding(&mut self, event: InputEvent, code: u16, value: i32) -> Vec<Action> {
        let (held_code, held_base) = match &self.state {
            State::Holding {
                key_code, base, ..
            } => (*key_code, base.clone()),
            _ => unreachable!(),
        };

        // Key repeat of the held key: suppress (we already emitted the first press)
        if code == held_code && value == 2 {
            return vec![Action::Suppress];
        }

        // Release of the held key: cancel timer, go idle
        if code == held_code && value == 0 {
            debug!(base = %held_base, "hold cancelled: key released before threshold");
            self.state = State::Idle;
            return vec![Action::Relay(event)];
        }

        // Any other key press: cancel timer, relay both
        if value == 1 {
            debug!(base = %held_base, other_key = code, "hold cancelled: another key pressed");
            self.state = State::Idle;
            return vec![Action::Relay(event)];
        }

        // Other events: relay
        vec![Action::Relay(event)]
    }

    fn handle_popup(&mut self, event: InputEvent, code: u16, value: i32) -> Vec<Action> {
        let (popup_accents, popup_code) = match &self.state {
            State::Popup { accents, key_code, .. } => (accents.clone(), *key_code),
            _ => unreachable!(),
        };

        // Key repeat of the held key: suppress
        if code == popup_code && value == 2 {
            return vec![Action::Suppress];
        }

        // Release of the held key: dismiss popup, keep base char
        if code == popup_code && value == 0 {
            debug!("popup dismissed: held key released");
            self.state = State::Idle;
            return vec![
                Action::SendPopup(DaemonMsg::HidePopup),
                Action::Suppress, // don't relay the release
            ];
        }

        // ESC press: dismiss popup
        if code == Key::KEY_ESC.code() && value == 1 {
            debug!("popup dismissed: ESC pressed");
            self.state = State::Idle;
            return vec![
                Action::SendPopup(DaemonMsg::HidePopup),
                Action::Suppress,
            ];
        }

        // Number key press: select accent
        if value == 1 {
            if let Some(digit) = charmap::keycode_to_digit(code) {
                let idx = (digit - 1) as usize;
                if idx < popup_accents.len() {
                    let accent = popup_accents[idx].clone();
                    debug!(accent = %accent, index = digit, "accent selected");
                    self.state = State::Idle;
                    return vec![
                        Action::SendPopup(DaemonMsg::HidePopup),
                        Action::EmitAccent(accent),
                    ];
                }
            }
        }

        // Any other key: dismiss popup and relay
        if value == 1 {
            debug!(code, "popup dismissed: unrelated key pressed");
            self.state = State::Idle;
            return vec![
                Action::SendPopup(DaemonMsg::HidePopup),
                Action::Relay(event),
            ];
        }

        vec![Action::Suppress]
    }

    #[cfg(test)]
    fn is_idle(&self) -> bool {
        self.state == State::Idle
    }

    fn update_modifiers(&mut self, event: &InputEvent) {
        let code = event.code();
        let pressed = event.value() == 1;
        let released = event.value() == 0;

        match Key::new(code) {
            Key::KEY_LEFTCTRL | Key::KEY_RIGHTCTRL => {
                if pressed {
                    self.ctrl_held = true;
                } else if released {
                    self.ctrl_held = false;
                }
            }
            Key::KEY_LEFTALT | Key::KEY_RIGHTALT => {
                if pressed {
                    self.alt_held = true;
                } else if released {
                    self.alt_held = false;
                }
            }
            Key::KEY_LEFTMETA | Key::KEY_RIGHTMETA => {
                if pressed {
                    self.super_held = true;
                } else if released {
                    self.super_held = false;
                }
            }
            Key::KEY_LEFTSHIFT | Key::KEY_RIGHTSHIFT => {
                if pressed {
                    self.shift_held = true;
                } else if released {
                    self.shift_held = false;
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use accentd_core::charmap::builtin_locale;

    // Helpers to create events without caring about implementation details.
    // evdev keycodes from Linux input-event-codes.h:
    const KEY_A: u16 = 30;
    const KEY_E: u16 = 18;
    const KEY_F: u16 = 33; // not accent-eligible
    const KEY_1: u16 = 2;
    const KEY_2: u16 = 3;
    const KEY_9: u16 = 10;
    const KEY_ESC: u16 = 1;
    const KEY_LEFTCTRL: u16 = 29;
    const KEY_LEFTALT: u16 = 56;
    const KEY_LEFTMETA: u16 = 125;
    const KEY_LEFTSHIFT: u16 = 42;

    fn key_press(code: u16) -> InputEvent {
        InputEvent::new(EventType::KEY, code, 1)
    }
    fn key_release(code: u16) -> InputEvent {
        InputEvent::new(EventType::KEY, code, 0)
    }
    fn key_repeat(code: u16) -> InputEvent {
        InputEvent::new(EventType::KEY, code, 2)
    }

    fn make_sm() -> StateMachine {
        let config = Config::default(); // threshold=300ms, enabled=true, locale=it
        let locale_map = builtin_locale("it");
        StateMachine::new(&config, locale_map)
    }

    fn has_relay(actions: &[Action]) -> bool {
        actions.iter().any(|a| matches!(a, Action::Relay(_)))
    }
    fn has_suppress(actions: &[Action]) -> bool {
        actions.iter().any(|a| matches!(a, Action::Suppress))
    }
    fn has_emit_accent(actions: &[Action]) -> Option<&str> {
        actions.iter().find_map(|a| match a {
            Action::EmitAccent(s) => Some(s.as_str()),
            _ => None,
        })
    }
    fn has_show_popup(actions: &[Action]) -> bool {
        actions.iter().any(|a| matches!(a, Action::SendPopup(DaemonMsg::ShowPopup { .. })))
    }
    fn has_hide_popup(actions: &[Action]) -> bool {
        actions.iter().any(|a| matches!(a, Action::SendPopup(DaemonMsg::HidePopup)))
    }

    // === SPEC: Press accent key → emit immediately (zero latency) ===

    #[test]
    fn press_accent_key_relays_immediately() {
        let mut sm = make_sm();
        let actions = sm.process_event(key_press(KEY_E));
        assert!(has_relay(&actions), "pressing 'e' should relay immediately");
    }

    // === SPEC: Press non-accent key → relay normally, no hold ===

    #[test]
    fn press_non_accent_key_relays_and_stays_idle() {
        let mut sm = make_sm();
        let actions = sm.process_event(key_press(KEY_F));
        assert!(has_relay(&actions));
        assert!(sm.is_idle());
    }

    // === SPEC: Release before threshold → cancel, base char stays ===

    #[test]
    fn release_before_threshold_cancels_hold() {
        let mut sm = make_sm();
        sm.process_event(key_press(KEY_E));
        let actions = sm.process_event(key_release(KEY_E));
        assert!(has_relay(&actions), "release should be relayed");
        assert!(sm.is_idle(), "should return to idle");
    }

    // === SPEC: Another key before threshold → cancel hold ===

    #[test]
    fn another_key_before_threshold_cancels_hold() {
        let mut sm = make_sm();
        sm.process_event(key_press(KEY_E));
        // Type another key quickly (fast typist)
        let actions = sm.process_event(key_press(KEY_F));
        assert!(has_relay(&actions), "other key should be relayed");
        assert!(sm.is_idle(), "should return to idle");
    }

    // === SPEC: Key repeat while holding → suppressed ===

    #[test]
    fn repeat_during_hold_is_suppressed() {
        let mut sm = make_sm();
        sm.process_event(key_press(KEY_E));
        let actions = sm.process_event(key_repeat(KEY_E));
        assert!(has_suppress(&actions), "repeat should be suppressed");
        assert!(!has_relay(&actions), "repeat should NOT be relayed");
    }

    // === SPEC: Hold past threshold → popup appears ===

    #[test]
    fn hold_past_threshold_shows_popup() {
        let mut sm = make_sm();
        sm.process_event(key_press(KEY_E));
        std::thread::sleep(std::time::Duration::from_millis(350));
        let actions = sm.check_timer();
        assert!(has_show_popup(&actions), "popup should appear after threshold");
        assert!(has_relay(&actions), "should emit synthetic key release to stop autorepeat");
        assert!(!sm.is_idle(), "should not be idle (should be in popup state)");
    }

    // === SPEC: Timer doesn't fire before threshold ===

    #[test]
    fn timer_does_not_fire_before_threshold() {
        let mut sm = make_sm();
        sm.process_event(key_press(KEY_E));
        let actions = sm.check_timer();
        assert!(actions.is_empty(), "timer should not fire immediately");
    }

    // === SPEC: Threshold transition must release key to stop display server autorepeat ===

    #[test]
    fn threshold_emits_key_release_to_stop_autorepeat() {
        let mut sm = make_sm();
        sm.process_event(key_press(KEY_E));
        std::thread::sleep(std::time::Duration::from_millis(350));
        let actions = sm.check_timer();

        // Must contain a release event (value=0) for KEY_E before the ShowPopup
        let release_idx = actions.iter().position(|a| match a {
            Action::Relay(ev) => ev.code() == KEY_E && ev.value() == 0,
            _ => false,
        });
        let popup_idx = actions.iter().position(|a| matches!(a, Action::SendPopup(DaemonMsg::ShowPopup { .. })));

        assert!(release_idx.is_some(), "must emit key release to stop autorepeat");
        assert!(popup_idx.is_some(), "must show popup");
        assert!(release_idx.unwrap() < popup_idx.unwrap(), "release must come before popup");
    }

    // === SPEC: In popup, press digit → select accent ===

    #[test]
    fn popup_digit_selects_accent() {
        let mut sm = make_sm();
        sm.process_event(key_press(KEY_E));
        std::thread::sleep(std::time::Duration::from_millis(350));
        sm.check_timer(); // triggers popup

        // Press '1' to select first accent (è for Italian 'e')
        let actions = sm.process_event(key_press(KEY_1));
        assert!(has_hide_popup(&actions), "popup should hide");
        assert_eq!(has_emit_accent(&actions), Some("è"), "should emit first Italian 'e' accent");
        assert!(sm.is_idle());
    }

    #[test]
    fn popup_digit_2_selects_second_accent() {
        let mut sm = make_sm();
        sm.process_event(key_press(KEY_E));
        std::thread::sleep(std::time::Duration::from_millis(350));
        sm.check_timer();

        let actions = sm.process_event(key_press(KEY_2));
        assert_eq!(has_emit_accent(&actions), Some("é"), "should emit second Italian 'e' accent");
    }

    // === SPEC: In popup, out-of-range digit → no accent emitted ===

    #[test]
    fn popup_out_of_range_digit_dismisses() {
        let mut sm = make_sm();
        sm.process_event(key_press(KEY_E));
        std::thread::sleep(std::time::Duration::from_millis(350));
        sm.check_timer();

        // Italian 'e' has 4 accents, press 9
        let actions = sm.process_event(key_press(KEY_9));
        assert!(has_emit_accent(&actions).is_none(), "should not emit accent for out-of-range digit");
        assert!(has_hide_popup(&actions), "popup should dismiss");
    }

    // === SPEC: In popup, ESC → dismiss, base char stays ===

    #[test]
    fn popup_esc_dismisses() {
        let mut sm = make_sm();
        sm.process_event(key_press(KEY_E));
        std::thread::sleep(std::time::Duration::from_millis(350));
        sm.check_timer();

        let actions = sm.process_event(key_press(KEY_ESC));
        assert!(has_hide_popup(&actions));
        assert!(has_emit_accent(&actions).is_none(), "ESC should not emit any accent");
        assert!(sm.is_idle());
    }

    // === SPEC: In popup, release held key → dismiss ===

    #[test]
    fn popup_release_held_key_dismisses() {
        let mut sm = make_sm();
        sm.process_event(key_press(KEY_E));
        std::thread::sleep(std::time::Duration::from_millis(350));
        sm.check_timer();

        let actions = sm.process_event(key_release(KEY_E));
        assert!(has_hide_popup(&actions));
        assert!(has_emit_accent(&actions).is_none());
        assert!(sm.is_idle());
    }

    // === SPEC: In popup, repeat of held key → suppressed ===

    #[test]
    fn popup_repeat_held_key_is_suppressed() {
        let mut sm = make_sm();
        sm.process_event(key_press(KEY_E));
        std::thread::sleep(std::time::Duration::from_millis(350));
        sm.check_timer();

        let actions = sm.process_event(key_repeat(KEY_E));
        assert!(has_suppress(&actions));
        assert!(!has_relay(&actions));
    }

    // === SPEC: Ctrl/Alt/Super + letter → no hold, relay as-is ===

    #[test]
    fn ctrl_plus_letter_does_not_start_hold() {
        let mut sm = make_sm();
        sm.process_event(key_press(KEY_LEFTCTRL));
        sm.process_event(key_press(KEY_A));
        assert!(sm.is_idle(), "Ctrl+A should not start hold detection");
    }

    #[test]
    fn alt_plus_letter_does_not_start_hold() {
        let mut sm = make_sm();
        sm.process_event(key_press(KEY_LEFTALT));
        sm.process_event(key_press(KEY_E));
        assert!(sm.is_idle());
    }

    #[test]
    fn super_plus_letter_does_not_start_hold() {
        let mut sm = make_sm();
        sm.process_event(key_press(KEY_LEFTMETA));
        sm.process_event(key_press(KEY_E));
        assert!(sm.is_idle());
    }

    // === SPEC: Shift + letter → uppercase accents ===

    #[test]
    fn shift_plus_letter_produces_uppercase_accents() {
        let mut sm = make_sm();
        sm.process_event(key_press(KEY_LEFTSHIFT));
        sm.process_event(key_press(KEY_E));
        std::thread::sleep(std::time::Duration::from_millis(350));
        let timer_actions = sm.check_timer();
        assert!(has_show_popup(&timer_actions));

        // Check the popup contains uppercase accents
        let show = timer_actions.iter().find_map(|a| match a {
            Action::SendPopup(DaemonMsg::ShowPopup { accents, .. }) => Some(accents),
            _ => None,
        }).expect("should have ShowPopup");
        assert_eq!(show[0], "È");
        assert_eq!(show[1], "É");
    }

    // === SPEC: Disabled → all keys relayed, no hold ===

    #[test]
    fn disabled_relays_everything() {
        let mut sm = make_sm();
        sm.set_enabled(false);
        let actions = sm.process_event(key_press(KEY_E));
        assert!(has_relay(&actions));
        assert!(sm.is_idle());
    }

    // === SPEC: Non-key events always relayed ===

    #[test]
    fn non_key_events_always_relayed() {
        let mut sm = make_sm();
        // EV_SYN event
        let syn = InputEvent::new(EventType::SYNCHRONIZATION, 0, 0);
        let actions = sm.process_event(syn);
        assert!(has_relay(&actions));
    }

    // === SPEC: ShowPopup labels are 1-indexed ===

    #[test]
    fn show_popup_labels_are_1_indexed() {
        let mut sm = make_sm();
        sm.process_event(key_press(KEY_E));
        std::thread::sleep(std::time::Duration::from_millis(350));
        let actions = sm.check_timer();
        let labels = actions.iter().find_map(|a| match a {
            Action::SendPopup(DaemonMsg::ShowPopup { labels, .. }) => Some(labels),
            _ => None,
        }).expect("should have ShowPopup");
        assert_eq!(labels[0], 1);
        assert_eq!(*labels.last().unwrap(), labels.len() as u8);
    }

    // === SPEC: 'a' key with Italian locale has 5 accents ===

    #[test]
    fn italian_a_popup_has_5_accents() {
        let mut sm = make_sm();
        sm.process_event(key_press(KEY_A));
        std::thread::sleep(std::time::Duration::from_millis(350));
        let actions = sm.check_timer();
        let accents = actions.iter().find_map(|a| match a {
            Action::SendPopup(DaemonMsg::ShowPopup { accents, .. }) => Some(accents),
            _ => None,
        }).expect("should have ShowPopup");
        assert_eq!(accents.len(), 5);
        assert_eq!(accents[0], "à");
    }

    // === SPEC: Modifier released → subsequent letters should hold ===

    #[test]
    fn modifier_release_allows_hold_again() {
        let mut sm = make_sm();
        // Press and release Ctrl
        sm.process_event(key_press(KEY_LEFTCTRL));
        sm.process_event(key_release(KEY_LEFTCTRL));
        // Now press 'e' — should start hold
        sm.process_event(key_press(KEY_E));
        assert!(!sm.is_idle(), "should be in holding state after modifier released");
    }

    // === SPEC: IPC select → emit accent if in popup state ===

    fn enter_popup(sm: &mut StateMachine) {
        sm.process_event(key_press(KEY_E));
        std::thread::sleep(std::time::Duration::from_millis(350));
        sm.check_timer();
    }

    #[test]
    fn ipc_select_emits_accent_in_popup_state() {
        let mut sm = make_sm();
        enter_popup(&mut sm);

        let actions = sm.ipc_select(1);
        assert!(has_hide_popup(&actions));
        assert_eq!(has_emit_accent(&actions), Some("è"));
        assert!(sm.is_idle());
    }

    #[test]
    fn ipc_select_returns_empty_when_idle() {
        let mut sm = make_sm();
        let actions = sm.ipc_select(1);
        assert!(actions.is_empty());
    }

    // === SPEC: IPC dismiss → hide popup if in popup state ===

    #[test]
    fn ipc_dismiss_hides_popup() {
        let mut sm = make_sm();
        enter_popup(&mut sm);

        let actions = sm.ipc_dismiss();
        assert!(has_hide_popup(&actions));
        assert!(has_emit_accent(&actions).is_none());
        assert!(sm.is_idle());
    }

    #[test]
    fn ipc_dismiss_returns_empty_when_idle() {
        let mut sm = make_sm();
        let actions = sm.ipc_dismiss();
        assert!(actions.is_empty());
    }

    // === SPEC: Popup auto-timeout ===

    #[test]
    fn popup_times_out_after_timeout_ms() {
        let mut config = Config::default();
        config.popup.timeout_ms = 50; // short timeout for test
        let locale_map = builtin_locale("it");
        let mut sm = StateMachine::new(&config, locale_map);

        sm.process_event(key_press(KEY_E));
        std::thread::sleep(std::time::Duration::from_millis(350));
        sm.check_timer(); // enters popup

        std::thread::sleep(std::time::Duration::from_millis(60));
        let actions = sm.check_timer();
        assert!(has_hide_popup(&actions), "popup should auto-dismiss after timeout");
        assert!(sm.is_idle());
    }

    // === SPEC: next_deadline ===

    #[test]
    fn next_deadline_is_none_when_idle() {
        let sm = make_sm();
        assert!(sm.next_deadline().is_none());
    }

    #[test]
    fn next_deadline_is_some_when_holding() {
        let mut sm = make_sm();
        sm.process_event(key_press(KEY_E));
        assert!(sm.next_deadline().is_some());
    }

    #[test]
    fn next_deadline_is_some_when_popup() {
        let mut sm = make_sm();
        enter_popup(&mut sm);
        assert!(sm.next_deadline().is_some());
    }
}

