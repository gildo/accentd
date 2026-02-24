use std::collections::HashMap;

/// Return the built-in accent map for a locale.
pub fn builtin_locale(name: &str) -> HashMap<String, Vec<String>> {
    match name {
        "it" => locale_it(),
        "es" => locale_es(),
        "fr" => locale_fr(),
        "de" => locale_de(),
        "pt" => locale_pt(),
        _ => HashMap::new(),
    }
}

fn locale_it() -> HashMap<String, Vec<String>> {
    HashMap::from([
        ("a".into(), vec!["à", "á", "â", "ã", "ä"].into_iter().map(Into::into).collect()),
        ("e".into(), vec!["è", "é", "ê", "ë"].into_iter().map(Into::into).collect()),
        ("i".into(), vec!["ì", "í", "î", "ï"].into_iter().map(Into::into).collect()),
        ("o".into(), vec!["ò", "ó", "ô", "õ", "ö"].into_iter().map(Into::into).collect()),
        ("u".into(), vec!["ù", "ú", "û", "ü"].into_iter().map(Into::into).collect()),
        ("n".into(), vec!["ñ"].into_iter().map(Into::into).collect()),
        ("c".into(), vec!["ç"].into_iter().map(Into::into).collect()),
    ])
}

fn locale_es() -> HashMap<String, Vec<String>> {
    HashMap::from([
        ("a".into(), vec!["á", "à", "â", "ä"].into_iter().map(Into::into).collect()),
        ("e".into(), vec!["é", "è", "ê", "ë"].into_iter().map(Into::into).collect()),
        ("i".into(), vec!["í", "ì", "î", "ï"].into_iter().map(Into::into).collect()),
        ("o".into(), vec!["ó", "ò", "ô", "ö"].into_iter().map(Into::into).collect()),
        ("u".into(), vec!["ú", "ù", "û", "ü"].into_iter().map(Into::into).collect()),
        ("n".into(), vec!["ñ"].into_iter().map(Into::into).collect()),
        ("y".into(), vec!["ý", "ÿ"].into_iter().map(Into::into).collect()),
    ])
}

fn locale_fr() -> HashMap<String, Vec<String>> {
    HashMap::from([
        ("a".into(), vec!["à", "â", "æ", "á", "ä"].into_iter().map(Into::into).collect()),
        ("e".into(), vec!["è", "é", "ê", "ë", "æ"].into_iter().map(Into::into).collect()),
        ("i".into(), vec!["î", "ï", "í", "ì"].into_iter().map(Into::into).collect()),
        ("o".into(), vec!["ô", "œ", "ö", "ò", "ó"].into_iter().map(Into::into).collect()),
        ("u".into(), vec!["ù", "û", "ü", "ú"].into_iter().map(Into::into).collect()),
        ("c".into(), vec!["ç"].into_iter().map(Into::into).collect()),
        ("y".into(), vec!["ÿ"].into_iter().map(Into::into).collect()),
    ])
}

fn locale_de() -> HashMap<String, Vec<String>> {
    HashMap::from([
        ("a".into(), vec!["ä", "à", "á", "â"].into_iter().map(Into::into).collect()),
        ("e".into(), vec!["ë", "è", "é", "ê"].into_iter().map(Into::into).collect()),
        ("i".into(), vec!["ï", "ì", "í", "î"].into_iter().map(Into::into).collect()),
        ("o".into(), vec!["ö", "ò", "ó", "ô"].into_iter().map(Into::into).collect()),
        ("u".into(), vec!["ü", "ù", "ú", "û"].into_iter().map(Into::into).collect()),
        ("s".into(), vec!["ß"].into_iter().map(Into::into).collect()),
    ])
}

fn locale_pt() -> HashMap<String, Vec<String>> {
    HashMap::from([
        ("a".into(), vec!["ã", "á", "à", "â", "ä"].into_iter().map(Into::into).collect()),
        ("e".into(), vec!["é", "è", "ê", "ë"].into_iter().map(Into::into).collect()),
        ("i".into(), vec!["í", "ì", "î", "ï"].into_iter().map(Into::into).collect()),
        ("o".into(), vec!["õ", "ó", "ò", "ô", "ö"].into_iter().map(Into::into).collect()),
        ("u".into(), vec!["ú", "ù", "û", "ü"].into_iter().map(Into::into).collect()),
        ("c".into(), vec!["ç"].into_iter().map(Into::into).collect()),
    ])
}

/// Given a lowercase base char and shift state, return the accented variants.
/// If shift is true, returns uppercase variants.
pub fn resolve_accents(
    locale_map: &HashMap<String, Vec<String>>,
    base: &str,
    shift: bool,
) -> Option<Vec<String>> {
    let lower = base.to_lowercase();
    let accents = locale_map.get(&lower)?;
    if shift {
        Some(
            accents
                .iter()
                .map(|s| s.to_uppercase())
                .collect(),
        )
    } else {
        Some(accents.clone())
    }
}

/// Map evdev key codes to base letter names.
/// Returns None for keys that are not accent-eligible.
/// NOTE: These keycodes assume a QWERTY physical layout. Non-QWERTY layouts
/// (Dvorak, AZERTY, Colemak) will map to wrong letters.
pub fn keycode_to_base(code: u16) -> Option<&'static str> {
    match code {
        30 => Some("a"),
        46 => Some("c"),
        18 => Some("e"),
        23 => Some("i"),
        49 => Some("n"),
        24 => Some("o"),
        31 => Some("s"),
        22 => Some("u"),
        21 => Some("y"),
        _ => None,
    }
}

/// Check if a keycode maps to a digit 1-9 (for accent selection).
/// Returns the 1-indexed number, or None.
pub fn keycode_to_digit(code: u16) -> Option<u8> {
    match code {
        2 => Some(1),   // KEY_1
        3 => Some(2),   // KEY_2
        4 => Some(3),   // KEY_3
        5 => Some(4),   // KEY_4
        6 => Some(5),   // KEY_5
        7 => Some(6),   // KEY_6
        8 => Some(7),   // KEY_7
        9 => Some(8),   // KEY_8
        10 => Some(9),  // KEY_9
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- spec: only a, e, i, o, u, n, c, s, y are accent-eligible ---

    #[test]
    fn accent_eligible_keys_map_to_correct_base() {
        // evdev keycodes: A=30, C=46, E=18, I=23, N=49, O=24, S=31, U=22, Y=21
        assert_eq!(keycode_to_base(30), Some("a"));
        assert_eq!(keycode_to_base(46), Some("c"));
        assert_eq!(keycode_to_base(18), Some("e"));
        assert_eq!(keycode_to_base(23), Some("i"));
        assert_eq!(keycode_to_base(49), Some("n"));
        assert_eq!(keycode_to_base(24), Some("o"));
        assert_eq!(keycode_to_base(31), Some("s"));
        assert_eq!(keycode_to_base(22), Some("u"));
        assert_eq!(keycode_to_base(21), Some("y"));
    }

    #[test]
    fn non_accent_keys_return_none() {
        // b=48, d=32, f=33, g=34, h=35, j=36, k=37, l=38
        for code in [48, 32, 33, 34, 35, 36, 37, 38, 0, 255] {
            assert_eq!(keycode_to_base(code), None, "keycode {} should not be accent-eligible", code);
        }
    }

    // --- spec: digit keys 1-9 for accent selection ---

    #[test]
    fn digit_keys_map_to_1_through_9() {
        for (code, expected) in (2..=10).zip(1u8..=9) {
            assert_eq!(keycode_to_digit(code), Some(expected));
        }
    }

    #[test]
    fn non_digit_keys_return_none() {
        assert_eq!(keycode_to_digit(0), None);
        assert_eq!(keycode_to_digit(1), None);  // KEY_ESC
        assert_eq!(keycode_to_digit(11), None); // KEY_0
        assert_eq!(keycode_to_digit(30), None); // KEY_A
    }

    // --- spec: Italian locale has specific accent orderings ---

    #[test]
    fn italian_locale_has_all_expected_keys() {
        let it = builtin_locale("it");
        assert!(it.contains_key("a"));
        assert!(it.contains_key("e"));
        assert!(it.contains_key("i"));
        assert!(it.contains_key("o"));
        assert!(it.contains_key("u"));
        assert!(it.contains_key("n"));
        assert!(it.contains_key("c"));
    }

    #[test]
    fn italian_e_has_grave_first() {
        // spec: e = ["è", "é", "ê", "ë"]
        let it = builtin_locale("it");
        let e_accents = &it["e"];
        assert_eq!(e_accents[0], "è");
        assert_eq!(e_accents[1], "é");
        assert_eq!(e_accents.len(), 4);
    }

    #[test]
    fn italian_a_has_5_variants() {
        // spec: a = ["à", "á", "â", "ã", "ä"]
        let it = builtin_locale("it");
        assert_eq!(it["a"].len(), 5);
        assert_eq!(it["a"][0], "à");
    }

    #[test]
    fn italian_n_has_only_tilde() {
        let it = builtin_locale("it");
        assert_eq!(it["n"], vec!["ñ"]);
    }

    #[test]
    fn italian_c_has_only_cedilla() {
        let it = builtin_locale("it");
        assert_eq!(it["c"], vec!["ç"]);
    }

    // --- spec: German has s → ß ---

    #[test]
    fn german_s_has_eszett() {
        let de = builtin_locale("de");
        assert_eq!(de["s"], vec!["ß"]);
    }

    #[test]
    fn german_has_umlauts_first() {
        // spec: German prioritizes umlauts
        let de = builtin_locale("de");
        assert_eq!(de["a"][0], "ä");
        assert_eq!(de["o"][0], "ö");
        assert_eq!(de["u"][0], "ü");
    }

    // --- spec: all 5 locales exist ---

    #[test]
    fn all_five_locales_exist_and_are_non_empty() {
        for name in ["it", "es", "fr", "de", "pt"] {
            let locale = builtin_locale(name);
            assert!(!locale.is_empty(), "locale '{}' should not be empty", name);
        }
    }

    #[test]
    fn unknown_locale_returns_empty() {
        assert!(builtin_locale("zz").is_empty());
        assert!(builtin_locale("").is_empty());
    }

    // --- spec: shift → uppercase variants ---

    #[test]
    fn resolve_accents_returns_lowercase_when_no_shift() {
        let it = builtin_locale("it");
        let accents = resolve_accents(&it, "e", false).unwrap();
        assert_eq!(accents[0], "è");
        assert_eq!(accents[1], "é");
    }

    #[test]
    fn resolve_accents_returns_uppercase_when_shift() {
        let it = builtin_locale("it");
        let accents = resolve_accents(&it, "e", true).unwrap();
        assert_eq!(accents[0], "È");
        assert_eq!(accents[1], "É");
    }

    #[test]
    fn resolve_accents_returns_none_for_unknown_base() {
        let it = builtin_locale("it");
        assert!(resolve_accents(&it, "z", false).is_none());
        assert!(resolve_accents(&it, "b", false).is_none());
    }

    // --- spec: Spanish has ñ and ý ---

    #[test]
    fn spanish_has_n_tilde_and_y_accents() {
        let es = builtin_locale("es");
        assert_eq!(es["n"], vec!["ñ"]);
        assert!(es.contains_key("y"));
        assert_eq!(es["y"][0], "ý");
    }

    // --- spec: French has æ, œ, ç ---

    #[test]
    fn french_has_ligatures_and_cedilla() {
        let fr = builtin_locale("fr");
        assert!(fr["a"].contains(&"æ".to_string()));
        assert!(fr["o"].contains(&"œ".to_string()));
        assert_eq!(fr["c"], vec!["ç"]);
    }

    // --- spec: Portuguese prioritizes tilde for a, o ---

    #[test]
    fn portuguese_a_starts_with_tilde() {
        let pt = builtin_locale("pt");
        assert_eq!(pt["a"][0], "ã");
    }

    #[test]
    fn portuguese_o_starts_with_tilde() {
        let pt = builtin_locale("pt");
        assert_eq!(pt["o"][0], "õ");
    }
}
