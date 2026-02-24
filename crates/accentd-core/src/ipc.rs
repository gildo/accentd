use serde::{Deserialize, Serialize};

/// Messages from daemon to popup/clients (JSON-lines over Unix socket).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DaemonMsg {
    /// Show the accent popup for the given base character.
    #[serde(rename = "show_popup")]
    ShowPopup {
        base: String,
        accents: Vec<String>,
        /// 1-indexed labels for display
        labels: Vec<u8>,
    },
    /// Hide the popup (user released key or pressed ESC).
    #[serde(rename = "hide_popup")]
    HidePopup,
    /// Status response.
    #[serde(rename = "status")]
    Status {
        enabled: bool,
        locale: String,
        version: String,
    },
    /// Acknowledgement for commands.
    #[serde(rename = "ack")]
    Ack { ok: bool, message: String },
}

/// Messages from popup/clients to daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMsg {
    /// User selected an accent variant (1-indexed).
    #[serde(rename = "select")]
    Select { index: u8 },
    /// User dismissed the popup.
    #[serde(rename = "dismiss")]
    Dismiss,
    /// Request to toggle enabled state.
    #[serde(rename = "toggle")]
    Toggle,
    /// Request to enable.
    #[serde(rename = "enable")]
    Enable,
    /// Request to disable.
    #[serde(rename = "disable")]
    Disable,
    /// Request to change locale.
    #[serde(rename = "set_locale")]
    SetLocale { locale: String },
    /// Request current status.
    #[serde(rename = "get_status")]
    GetStatus,
    /// Popup client announcing itself (for routing ShowPopup/HidePopup).
    #[serde(rename = "register_popup")]
    RegisterPopup,
}

/// Serialize a message as a JSON line (with trailing newline).
pub fn encode(msg: &impl Serialize) -> String {
    let mut s = serde_json::to_string(msg).expect("serialize IPC message");
    s.push('\n');
    s
}

/// Deserialize a JSON line. Returns None on empty/whitespace input.
pub fn decode_daemon(line: &str) -> Option<DaemonMsg> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    serde_json::from_str(trimmed).ok()
}

pub fn decode_client(line: &str) -> Option<ClientMsg> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    serde_json::from_str(trimmed).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- spec: encoded messages end with newline ---

    #[test]
    fn encode_produces_trailing_newline() {
        let msg = DaemonMsg::HidePopup;
        let encoded = encode(&msg);
        assert!(encoded.ends_with('\n'));
    }

    #[test]
    fn encode_produces_single_line() {
        let msg = DaemonMsg::ShowPopup {
            base: "e".into(),
            accents: vec!["è".into(), "é".into()],
            labels: vec![1, 2],
        };
        let encoded = encode(&msg);
        // Should be exactly one newline at the end
        assert_eq!(encoded.matches('\n').count(), 1);
    }

    // --- spec: encode then decode round-trips ---

    #[test]
    fn daemon_msg_hide_popup_round_trips() {
        let msg = DaemonMsg::HidePopup;
        let encoded = encode(&msg);
        let decoded = decode_daemon(&encoded).expect("should decode");
        assert!(matches!(decoded, DaemonMsg::HidePopup));
    }

    #[test]
    fn daemon_msg_show_popup_round_trips() {
        let msg = DaemonMsg::ShowPopup {
            base: "e".into(),
            accents: vec!["è".into(), "é".into(), "ê".into()],
            labels: vec![1, 2, 3],
        };
        let encoded = encode(&msg);
        let decoded = decode_daemon(&encoded).expect("should decode");
        match decoded {
            DaemonMsg::ShowPopup { base, accents, labels } => {
                assert_eq!(base, "e");
                assert_eq!(accents, vec!["è", "é", "ê"]);
                assert_eq!(labels, vec![1, 2, 3]);
            }
            _ => panic!("expected ShowPopup"),
        }
    }

    #[test]
    fn daemon_msg_status_round_trips() {
        let msg = DaemonMsg::Status {
            enabled: true,
            locale: "it".into(),
            version: "0.1.0".into(),
        };
        let encoded = encode(&msg);
        let decoded = decode_daemon(&encoded).expect("should decode");
        match decoded {
            DaemonMsg::Status { enabled, locale, version } => {
                assert!(enabled);
                assert_eq!(locale, "it");
                assert_eq!(version, "0.1.0");
            }
            _ => panic!("expected Status"),
        }
    }

    #[test]
    fn daemon_msg_ack_round_trips() {
        let msg = DaemonMsg::Ack { ok: false, message: "error".into() };
        let encoded = encode(&msg);
        let decoded = decode_daemon(&encoded).expect("should decode");
        match decoded {
            DaemonMsg::Ack { ok, message } => {
                assert!(!ok);
                assert_eq!(message, "error");
            }
            _ => panic!("expected Ack"),
        }
    }

    #[test]
    fn client_msg_select_round_trips() {
        let msg = ClientMsg::Select { index: 3 };
        let encoded = encode(&msg);
        let decoded = decode_client(&encoded).expect("should decode");
        match decoded {
            ClientMsg::Select { index } => assert_eq!(index, 3),
            _ => panic!("expected Select"),
        }
    }

    #[test]
    fn client_msg_set_locale_round_trips() {
        let msg = ClientMsg::SetLocale { locale: "fr".into() };
        let encoded = encode(&msg);
        let decoded = decode_client(&encoded).expect("should decode");
        match decoded {
            ClientMsg::SetLocale { locale } => assert_eq!(locale, "fr"),
            _ => panic!("expected SetLocale"),
        }
    }

    #[test]
    fn client_msg_simple_variants_round_trip() {
        for msg in [
            ClientMsg::Dismiss,
            ClientMsg::Toggle,
            ClientMsg::Enable,
            ClientMsg::Disable,
            ClientMsg::GetStatus,
            ClientMsg::RegisterPopup,
        ] {
            let encoded = encode(&msg);
            assert!(decode_client(&encoded).is_some(), "failed to round-trip: {:?}", msg);
        }
    }

    // --- spec: empty/whitespace input → None ---

    #[test]
    fn decode_daemon_returns_none_for_empty() {
        assert!(decode_daemon("").is_none());
        assert!(decode_daemon("   ").is_none());
        assert!(decode_daemon("\n").is_none());
    }

    #[test]
    fn decode_client_returns_none_for_empty() {
        assert!(decode_client("").is_none());
        assert!(decode_client("   ").is_none());
        assert!(decode_client("\n").is_none());
    }

    // --- spec: invalid JSON → None (not panic) ---

    #[test]
    fn decode_daemon_returns_none_for_garbage() {
        assert!(decode_daemon("not json").is_none());
        assert!(decode_daemon("{\"type\":\"unknown_variant\"}").is_none());
    }

    #[test]
    fn decode_client_returns_none_for_garbage() {
        assert!(decode_client("not json").is_none());
    }

    // --- spec: messages use the "type" tag ---

    #[test]
    fn encoded_messages_contain_type_field() {
        let encoded = encode(&DaemonMsg::HidePopup);
        assert!(encoded.contains("\"type\""));

        let encoded = encode(&ClientMsg::Toggle);
        assert!(encoded.contains("\"type\""));
    }
}
