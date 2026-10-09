use std::collections::HashMap;
use unicode_normalization::UnicodeNormalization;

/// Return the built-in accent map for a locale.
pub fn builtin_locale(name: &str) -> HashMap<String, Vec<String>> {
    match name {
        "it" => locale_it(),
        "es" => locale_es(),
        "fr" => locale_fr(),
        "de" => locale_de(),
        "pt" => locale_pt(),
        "macos" | "en" => macos_locale(),
        "unicode" => unicode_locale(),
        _ => HashMap::new(),
    }
}

/// Return the macOS U.S./ABC accent table.
///
/// Membership and order match the lists `PressAndHold.app` shows on macOS for
/// the U.S. keyboard layout, so the numbered selection keys line up with a Mac.
/// The data comes from the `Roman-Accent-*` entries in Apple's
/// `Keyboard-en.plist`.
pub fn macos_locale() -> HashMap<String, Vec<String>> {
    HashMap::from([
        (
            "a".into(),
            accents(&["à", "á", "â", "ä", "æ", "ã", "å", "ā"]),
        ),
        ("c".into(), accents(&["ç", "ć", "č"])),
        ("e".into(), accents(&["è", "é", "ê", "ë", "ē", "ė", "ę"])),
        ("i".into(), accents(&["î", "ï", "í", "ī", "į", "ì"])),
        ("l".into(), accents(&["ł"])),
        ("n".into(), accents(&["ñ", "ń"])),
        (
            "o".into(),
            accents(&["ô", "ö", "ò", "ó", "œ", "ø", "ō", "õ"]),
        ),
        ("s".into(), accents(&["ß", "ś", "š"])),
        ("u".into(), accents(&["û", "ü", "ù", "ú", "ū"])),
        ("y".into(), accents(&["ÿ"])),
        ("z".into(), accents(&["ž", "ź", "ż"])),
    ])
}

fn accents(list: &[&str]) -> Vec<String> {
    list.iter().map(|symbol| symbol.to_string()).collect()
}

/// The ten diacritics used by the `unicode` locale, in fixed slot order.
///
/// Slot labels run `1` through `9`, then `0` for the tenth.
const UNICODE_DIACRITICS: [char; 10] = [
    '\u{0300}', // grave
    '\u{0301}', // acute
    '\u{0302}', // circumflex
    '\u{0303}', // tilde
    '\u{0308}', // diaeresis
    '\u{030a}', // ring above
    '\u{0304}', // macron
    '\u{0306}', // breve
    '\u{030c}', // caron
    '\u{0327}', // cedilla
];

/// Return the `unicode` accent table for every Latin letter.
///
/// Each entry is a fixed-length slot vector: index `i` holds the variant for
/// `UNICODE_DIACRITICS[i]`, or an empty string when the letter has no such
/// precomposed form. This keeps each accent on the same number key across
/// letters, at the cost of holes for letters that lack an accent.
///
/// Variants are produced from Unicode canonical composition, so the table
/// covers every precomposed Latin letter rather than a hand-written list.
pub fn unicode_locale() -> HashMap<String, Vec<String>> {
    ('a'..='z')
        .filter_map(|base| {
            let slots: Vec<String> = UNICODE_DIACRITICS
                .iter()
                .map(|&mark| compose(base, mark).unwrap_or_default())
                .collect();
            slots
                .iter()
                .any(|slot| !slot.is_empty())
                .then(|| (base.to_string(), slots))
        })
        .collect()
}

/// Compose `base` with a single combining `mark`, if Unicode has a precomposed
/// character for the pair.
fn compose(base: char, mark: char) -> Option<String> {
    let decomposed: String = [base, mark].iter().collect();
    let composed: String = decomposed.nfc().collect();
    let mut chars = composed.chars();
    match (chars.next(), chars.next()) {
        (Some(character), None) if character != base => Some(character.to_string()),
        _ => None,
    }
}

fn locale_it() -> HashMap<String, Vec<String>> {
    HashMap::from([
        (
            "a".into(),
            vec!["à", "á", "â", "ã", "ä"]
                .into_iter()
                .map(Into::into)
                .collect(),
        ),
        (
            "e".into(),
            vec!["è", "é", "ê", "ë"]
                .into_iter()
                .map(Into::into)
                .collect(),
        ),
        (
            "i".into(),
            vec!["ì", "í", "î", "ï"]
                .into_iter()
                .map(Into::into)
                .collect(),
        ),
        (
            "o".into(),
            vec!["ò", "ó", "ô", "õ", "ö"]
                .into_iter()
                .map(Into::into)
                .collect(),
        ),
        (
            "u".into(),
            vec!["ù", "ú", "û", "ü"]
                .into_iter()
                .map(Into::into)
                .collect(),
        ),
        ("n".into(), vec!["ñ"].into_iter().map(Into::into).collect()),
        ("c".into(), vec!["ç"].into_iter().map(Into::into).collect()),
    ])
}

fn locale_es() -> HashMap<String, Vec<String>> {
    HashMap::from([
        (
            "a".into(),
            vec!["á", "à", "â", "ä"]
                .into_iter()
                .map(Into::into)
                .collect(),
        ),
        (
            "e".into(),
            vec!["é", "è", "ê", "ë"]
                .into_iter()
                .map(Into::into)
                .collect(),
        ),
        (
            "i".into(),
            vec!["í", "ì", "î", "ï"]
                .into_iter()
                .map(Into::into)
                .collect(),
        ),
        (
            "o".into(),
            vec!["ó", "ò", "ô", "ö"]
                .into_iter()
                .map(Into::into)
                .collect(),
        ),
        (
            "u".into(),
            vec!["ú", "ù", "û", "ü"]
                .into_iter()
                .map(Into::into)
                .collect(),
        ),
        ("n".into(), vec!["ñ"].into_iter().map(Into::into).collect()),
        (
            "y".into(),
            vec!["ý", "ÿ"].into_iter().map(Into::into).collect(),
        ),
    ])
}

fn locale_fr() -> HashMap<String, Vec<String>> {
    HashMap::from([
        (
            "a".into(),
            vec!["à", "â", "æ", "á", "ä"]
                .into_iter()
                .map(Into::into)
                .collect(),
        ),
        (
            "e".into(),
            vec!["è", "é", "ê", "ë", "æ"]
                .into_iter()
                .map(Into::into)
                .collect(),
        ),
        (
            "i".into(),
            vec!["î", "ï", "í", "ì"]
                .into_iter()
                .map(Into::into)
                .collect(),
        ),
        (
            "o".into(),
            vec!["ô", "œ", "ö", "ò", "ó"]
                .into_iter()
                .map(Into::into)
                .collect(),
        ),
        (
            "u".into(),
            vec!["ù", "û", "ü", "ú"]
                .into_iter()
                .map(Into::into)
                .collect(),
        ),
        ("c".into(), vec!["ç"].into_iter().map(Into::into).collect()),
        ("y".into(), vec!["ÿ"].into_iter().map(Into::into).collect()),
    ])
}

fn locale_de() -> HashMap<String, Vec<String>> {
    HashMap::from([
        (
            "a".into(),
            vec!["ä", "à", "á", "â"]
                .into_iter()
                .map(Into::into)
                .collect(),
        ),
        (
            "e".into(),
            vec!["ë", "è", "é", "ê"]
                .into_iter()
                .map(Into::into)
                .collect(),
        ),
        (
            "i".into(),
            vec!["ï", "ì", "í", "î"]
                .into_iter()
                .map(Into::into)
                .collect(),
        ),
        (
            "o".into(),
            vec!["ö", "ò", "ó", "ô"]
                .into_iter()
                .map(Into::into)
                .collect(),
        ),
        (
            "u".into(),
            vec!["ü", "ù", "ú", "û"]
                .into_iter()
                .map(Into::into)
                .collect(),
        ),
        ("s".into(), vec!["ß"].into_iter().map(Into::into).collect()),
    ])
}

fn locale_pt() -> HashMap<String, Vec<String>> {
    HashMap::from([
        (
            "a".into(),
            vec!["ã", "á", "à", "â", "ä"]
                .into_iter()
                .map(Into::into)
                .collect(),
        ),
        (
            "e".into(),
            vec!["é", "è", "ê", "ë"]
                .into_iter()
                .map(Into::into)
                .collect(),
        ),
        (
            "i".into(),
            vec!["í", "ì", "î", "ï"]
                .into_iter()
                .map(Into::into)
                .collect(),
        ),
        (
            "o".into(),
            vec!["õ", "ó", "ò", "ô", "ö"]
                .into_iter()
                .map(Into::into)
                .collect(),
        ),
        (
            "u".into(),
            vec!["ú", "ù", "û", "ü"]
                .into_iter()
                .map(Into::into)
                .collect(),
        ),
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
        Some(accents.iter().map(|s| s.to_uppercase()).collect())
    } else {
        Some(accents.clone())
    }
}

/// Map evdev key codes to base letter names.
///
/// Returns `None` for keys that are not letters. Whether a letter starts a
/// hold is decided by the active locale map, not here.
///
/// NOTE: These keycodes assume a QWERTY physical layout. Non-QWERTY layouts
/// (Dvorak, AZERTY, Colemak) will map to wrong letters.
pub fn keycode_to_base(code: u16) -> Option<&'static str> {
    match code {
        30 => Some("a"),
        48 => Some("b"),
        46 => Some("c"),
        32 => Some("d"),
        18 => Some("e"),
        33 => Some("f"),
        34 => Some("g"),
        35 => Some("h"),
        23 => Some("i"),
        36 => Some("j"),
        37 => Some("k"),
        38 => Some("l"),
        50 => Some("m"),
        49 => Some("n"),
        24 => Some("o"),
        25 => Some("p"),
        16 => Some("q"),
        19 => Some("r"),
        31 => Some("s"),
        20 => Some("t"),
        22 => Some("u"),
        47 => Some("v"),
        17 => Some("w"),
        45 => Some("x"),
        21 => Some("y"),
        44 => Some("z"),
        _ => None,
    }
}

/// Map a keycode to its accent-selection digit.
///
/// Returns `1`..`9` for the number row, `0` for the tenth slot, or `None` for
/// any other key.
pub fn keycode_to_digit(code: u16) -> Option<u8> {
    match code {
        2 => Some(1),  // KEY_1
        3 => Some(2),  // KEY_2
        4 => Some(3),  // KEY_3
        5 => Some(4),  // KEY_4
        6 => Some(5),  // KEY_5
        7 => Some(6),  // KEY_6
        8 => Some(7),  // KEY_7
        9 => Some(8),  // KEY_8
        10 => Some(9), // KEY_9
        11 => Some(0), // KEY_0, the tenth slot
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
    fn every_letter_key_maps_to_its_letter() {
        // QWERTY evdev keycodes for a..z.
        let expected = [
            (30, "a"),
            (48, "b"),
            (46, "c"),
            (32, "d"),
            (18, "e"),
            (33, "f"),
            (34, "g"),
            (35, "h"),
            (23, "i"),
            (36, "j"),
            (37, "k"),
            (38, "l"),
            (50, "m"),
            (49, "n"),
            (24, "o"),
            (25, "p"),
            (16, "q"),
            (19, "r"),
            (31, "s"),
            (20, "t"),
            (22, "u"),
            (47, "v"),
            (17, "w"),
            (45, "x"),
            (21, "y"),
            (44, "z"),
        ];
        for (code, letter) in expected {
            assert_eq!(keycode_to_base(code), Some(letter), "keycode {code}");
        }
    }

    #[test]
    fn non_letter_keys_return_none() {
        for code in [0, 11, 255] {
            assert_eq!(
                keycode_to_base(code),
                None,
                "keycode {code} is not a letter"
            );
        }
    }

    // --- spec: digit keys 1-9 and 0 for the tenth slot ---

    #[test]
    fn digit_keys_map_to_1_through_9_then_0() {
        for (code, expected) in (2..=10).zip(1u8..=9) {
            assert_eq!(keycode_to_digit(code), Some(expected));
        }
        assert_eq!(keycode_to_digit(11), Some(0));
    }

    #[test]
    fn non_digit_keys_return_none() {
        assert_eq!(keycode_to_digit(0), None);
        assert_eq!(keycode_to_digit(1), None); // KEY_ESC
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

    // --- spec: the macOS U.S. table and its order ---

    #[test]
    fn macos_locale_matches_apple_order() {
        let mac = macos_locale();
        assert_eq!(mac["a"], vec!["à", "á", "â", "ä", "æ", "ã", "å", "ā"]);
        assert_eq!(mac["c"], vec!["ç", "ć", "č"]);
        assert_eq!(mac["e"], vec!["è", "é", "ê", "ë", "ē", "ė", "ę"]);
        assert_eq!(mac["i"], vec!["î", "ï", "í", "ī", "į", "ì"]);
        assert_eq!(mac["l"], vec!["ł"]);
        assert_eq!(mac["n"], vec!["ñ", "ń"]);
        assert_eq!(mac["o"], vec!["ô", "ö", "ò", "ó", "œ", "ø", "ō", "õ"]);
        assert_eq!(mac["s"], vec!["ß", "ś", "š"]);
        assert_eq!(mac["u"], vec!["û", "ü", "ù", "ú", "ū"]);
        assert_eq!(mac["y"], vec!["ÿ"]);
        assert_eq!(mac["z"], vec!["ž", "ź", "ż"]);
    }

    // --- spec: the unicode table uses fixed slots with holes ---

    #[test]
    fn unicode_locale_places_accents_in_fixed_slots() {
        let table = unicode_locale();
        let a = &table["a"];
        assert_eq!(a.len(), 10);
        assert_eq!(a[0], "à");
        assert_eq!(a[1], "á");
        assert_eq!(a[2], "â");
        assert_eq!(a[3], "ã");
        assert_eq!(a[4], "ä");
        assert_eq!(a[5], "å");
        assert_eq!(a[6], "ā");
        assert_eq!(a[7], "ă");
        assert_eq!(a[8], "ǎ");
        assert_eq!(a[9], "");
    }

    #[test]
    fn unicode_locale_keeps_holes_for_missing_accents() {
        let table = unicode_locale();
        let c = &table["c"];
        assert_eq!(c[0], "");
        assert_eq!(c[1], "ć");
        assert_eq!(c[2], "ĉ");
        assert_eq!(c[8], "č");
        assert_eq!(c[9], "ç");
    }

    #[test]
    fn unicode_locale_omits_letters_without_variants() {
        let table = unicode_locale();
        assert!(!table.contains_key("b"));
    }

    #[test]
    fn unicode_locale_uppercases_with_holes() {
        let table = unicode_locale();
        let upper = resolve_accents(&table, "a", true).unwrap();
        assert_eq!(upper[0], "À");
        assert_eq!(upper[9], "");
    }

    #[test]
    fn unicode_locale_is_registered() {
        assert!(!builtin_locale("unicode").is_empty());
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
