//! A key pressed in the webview, as the screens expect it: a crossterm `KeyEvent`.
//!
//! The screens are written against crossterm's events, so the desktop shell hands them the same
//! type the terminal would. Text — typed characters, IME results, a paste — does not come through
//! here: xterm.js produces it and `ui/main.js` forwards it as text. Only named keys and keys held
//! with Ctrl, Alt or Super arrive as a `DomKey`.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use serde::Deserialize;

/// The fields of a DOM `KeyboardEvent` the conversion reads.
#[derive(Debug, Clone, Deserialize)]
pub struct DomKey {
    /// `KeyboardEvent.key`: `"a"`, `"K"`, `"Enter"`, `"ArrowUp"`, `"F5"`, …
    pub key: String,
    /// `KeyboardEvent.code`: the physical key, `"KeyK"`, `"Digit1"` — the same on every layout.
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub alt: bool,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub meta: bool,
}

/// `None` for a key the screens have no use for: a modifier on its own, an IME composition step,
/// a key the browser could not name.
pub fn to_key_event(dom: &DomKey) -> Option<KeyEvent> {
    let mut modifiers = KeyModifiers::empty();
    if dom.ctrl {
        modifiers |= KeyModifiers::CONTROL;
    }
    if dom.alt {
        modifiers |= KeyModifiers::ALT;
    }
    if dom.shift {
        modifiers |= KeyModifiers::SHIFT;
    }
    if dom.meta {
        modifiers |= KeyModifiers::SUPER;
    }

    let code = match dom.key.as_str() {
        "Enter" => KeyCode::Enter,
        "Escape" => KeyCode::Esc,
        "Backspace" => KeyCode::Backspace,
        // crossterm reports Shift+Tab as BackTab, and the screens match on that.
        "Tab" if dom.shift => KeyCode::BackTab,
        "Tab" => KeyCode::Tab,
        "Delete" => KeyCode::Delete,
        "Insert" => KeyCode::Insert,
        "Home" => KeyCode::Home,
        "End" => KeyCode::End,
        "PageUp" => KeyCode::PageUp,
        "PageDown" => KeyCode::PageDown,
        "ArrowUp" => KeyCode::Up,
        "ArrowDown" => KeyCode::Down,
        "ArrowLeft" => KeyCode::Left,
        "ArrowRight" => KeyCode::Right,
        " " => KeyCode::Char(' '),
        named if named.starts_with('F') && named.len() > 1 => {
            KeyCode::F(named[1..].parse().ok().filter(|n| (1..=24).contains(n))?)
        }
        other => {
            let mut chars = other.chars();
            match (chars.next(), chars.next()) {
                // Shortcuts are named by the Latin letter on the key. With a Cyrillic layout the
                // browser reports Ctrl+K as Ctrl+л, and no screen would ever match it.
                (Some(c), None) if !c.is_ascii() && (dom.ctrl || dom.alt || dom.meta) => {
                    KeyCode::Char(latin_on_key(&dom.code).unwrap_or(c))
                }
                (Some(c), None) => KeyCode::Char(c),
                // "Shift", "Control", "Dead", "Process", "Unidentified", …
                _ => return None,
            }
        }
    };

    Some(KeyEvent {
        code,
        modifiers,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

/// `"KeyK"` → `'k'`, `"Digit1"` → `'1'`; anything else has no single Latin character.
fn latin_on_key(code: &str) -> Option<char> {
    let rest = code
        .strip_prefix("Key")
        .or_else(|| code.strip_prefix("Digit"))?;
    let mut chars = rest.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii_alphanumeric() => Some(c.to_ascii_lowercase()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dom(key: &str) -> DomKey {
        DomKey {
            key: key.into(),
            code: String::new(),
            ctrl: false,
            alt: false,
            shift: false,
            meta: false,
        }
    }

    fn code(d: &DomKey) -> Option<KeyCode> {
        to_key_event(d).map(|e| e.code)
    }

    #[test]
    fn named_keys_become_their_crossterm_codes() {
        assert_eq!(code(&dom("Enter")), Some(KeyCode::Enter));
        assert_eq!(code(&dom("Escape")), Some(KeyCode::Esc));
        assert_eq!(code(&dom("ArrowUp")), Some(KeyCode::Up));
        assert_eq!(code(&dom("PageDown")), Some(KeyCode::PageDown));
        assert_eq!(code(&dom("F5")), Some(KeyCode::F(5)));
        assert_eq!(code(&dom("F12")), Some(KeyCode::F(12)));
    }

    #[test]
    fn shift_tab_is_backtab() {
        let shift_tab = DomKey {
            shift: true,
            ..dom("Tab")
        };
        assert_eq!(code(&shift_tab), Some(KeyCode::BackTab));
        assert_eq!(code(&dom("Tab")), Some(KeyCode::Tab));
    }

    #[test]
    fn a_held_modifier_travels_with_the_character() {
        let ctrl_k = DomKey {
            ctrl: true,
            ..dom("k")
        };
        let event = to_key_event(&ctrl_k).unwrap();
        assert_eq!(event.code, KeyCode::Char('k'));
        assert_eq!(event.modifiers, KeyModifiers::CONTROL);
        assert_eq!(event.kind, KeyEventKind::Press);
    }

    #[test]
    fn keys_the_screens_cannot_use_are_dropped() {
        for key in [
            "Shift",
            "Control",
            "Alt",
            "Meta",
            "Dead",
            "Process",
            "Unidentified",
            "F0",
            "Fx",
        ] {
            assert_eq!(code(&dom(key)), None, "{key}");
        }
    }

    #[test]
    fn a_capital_f_is_a_letter_not_a_function_key() {
        assert_eq!(code(&dom("F")), Some(KeyCode::Char('F')));
    }

    #[test]
    fn a_shortcut_on_a_cyrillic_layout_is_the_latin_letter() {
        let ctrl_k = DomKey {
            code: "KeyK".into(),
            ctrl: true,
            ..dom("л")
        };
        assert_eq!(code(&ctrl_k), Some(KeyCode::Char('k')));
    }

    #[test]
    fn cyrillic_without_a_modifier_stays_cyrillic() {
        let typed = DomKey {
            code: "KeyK".into(),
            ..dom("л")
        };
        assert_eq!(code(&typed), Some(KeyCode::Char('л')));
    }
}
