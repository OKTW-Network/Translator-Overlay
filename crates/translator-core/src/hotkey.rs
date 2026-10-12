//! Keyboard shortcuts stored in `config.toml`. No Win key, no mouse, no defaults.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// One non-modifier key plus optional Ctrl / Alt / Shift.
///
/// The Windows key is never stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HotkeyChord {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub vk: u16,
}

/// Which capture control a shortcut runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyAction {
    StartPause,
    Stop,
    SelectRegion,
    Retry,
    ClearHistory,
}

impl HotkeyAction {
    pub const ALL: [Self; 5] = [Self::StartPause, Self::Stop, Self::SelectRegion, Self::Retry, Self::ClearHistory];
}

/// Shortcut master switch, focus gate, and the five unbound-by-default chords.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HotkeyConfig {
    pub enabled: bool,
    pub only_when_target_focused: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_pause: Option<HotkeyChord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<HotkeyChord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select_region: Option<HotkeyChord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<HotkeyChord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clear_history: Option<HotkeyChord>,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            only_when_target_focused: true,
            start_pause: None,
            stop: None,
            select_region: None,
            retry: None,
            clear_history: None,
        }
    }
}

impl HotkeyConfig {
    pub fn get(&self, action: HotkeyAction) -> Option<HotkeyChord> {
        match action {
            HotkeyAction::StartPause => self.start_pause,
            HotkeyAction::Stop => self.stop,
            HotkeyAction::SelectRegion => self.select_region,
            HotkeyAction::Retry => self.retry,
            HotkeyAction::ClearHistory => self.clear_history,
        }
    }

    pub fn set(&mut self, action: HotkeyAction, chord: Option<HotkeyChord>) {
        let slot = match action {
            HotkeyAction::StartPause => &mut self.start_pause,
            HotkeyAction::Stop => &mut self.stop,
            HotkeyAction::SelectRegion => &mut self.select_region,
            HotkeyAction::Retry => &mut self.retry,
            HotkeyAction::ClearHistory => &mut self.clear_history,
        };
        *slot = chord;
    }

    /// First action in page order that uses `chord`.
    pub fn find(&self, chord: HotkeyChord) -> Option<HotkeyAction> {
        HotkeyAction::ALL.into_iter().find(|action| self.get(*action) == Some(chord))
    }

    /// Store `chord` on `action`. `Err` is the action that already owns it.
    pub fn assign(&mut self, action: HotkeyAction, chord: HotkeyChord) -> Result<(), HotkeyAction> {
        if let Some(other) = self.find(chord).filter(|other| *other != action) {
            return Err(other);
        }
        self.set(action, Some(chord));
        Ok(())
    }
}

const NAMES: &[(u16, &str, &str)] = &[
    (0x08, "backspace", "Backspace"),
    (0x09, "tab", "Tab"),
    (0x0D, "enter", "Enter"),
    (0x13, "pause", "Pause"),
    (0x14, "capslock", "Caps Lock"),
    (0x20, "space", "Space"),
    (0x21, "pageup", "Page Up"),
    (0x22, "pagedown", "Page Down"),
    (0x23, "end", "End"),
    (0x24, "home", "Home"),
    (0x25, "left", "Left"),
    (0x26, "up", "Up"),
    (0x27, "right", "Right"),
    (0x28, "down", "Down"),
    (0x2C, "printscreen", "Print Screen"),
    (0x2D, "insert", "Insert"),
    (0x2E, "delete", "Delete"),
    (0x5D, "apps", "Menu"),
    (0x60, "num0", "Num 0"),
    (0x61, "num1", "Num 1"),
    (0x62, "num2", "Num 2"),
    (0x63, "num3", "Num 3"),
    (0x64, "num4", "Num 4"),
    (0x65, "num5", "Num 5"),
    (0x66, "num6", "Num 6"),
    (0x67, "num7", "Num 7"),
    (0x68, "num8", "Num 8"),
    (0x69, "num9", "Num 9"),
    (0x6A, "nummul", "Num *"),
    (0x6B, "numadd", "Num +"),
    (0x6D, "numsub", "Num -"),
    (0x6E, "numdec", "Num ."),
    (0x6F, "numdiv", "Num /"),
    (0x90, "numlock", "Num Lock"),
    (0x91, "scrolllock", "Scroll Lock"),
    (0xAD, "mute", "Volume Mute"),
    (0xAE, "volumedown", "Volume Down"),
    (0xAF, "volumeup", "Volume Up"),
    (0xB0, "medianext", "Media Next"),
    (0xB1, "mediaprev", "Media Previous"),
    (0xB3, "mediaplay", "Media Play"),
    (0xBA, "semicolon", ";"),
    (0xBB, "equal", "="),
    (0xBC, "comma", ","),
    (0xBD, "minus", "-"),
    (0xBE, "period", "."),
    (0xBF, "slash", "/"),
    (0xC0, "backquote", "`"),
    (0xDB, "lbracket", "["),
    (0xDC, "backslash", "\\"),
    (0xDD, "rbracket", "]"),
    (0xDE, "quote", "'"),
];

const VK_ESCAPE: u16 = 0x1B;
const VK_PACKET: u16 = 0xE7;
const VK_PROCESSKEY: u16 = 0xE5;

pub fn is_win_vk(vk: u16) -> bool {
    vk == 0x5B || vk == 0x5C
}

fn is_mouse_vk(vk: u16) -> bool {
    matches!(vk, 0x01 | 0x02 | 0x04 | 0x05 | 0x06)
}

pub fn is_modifier_vk(vk: u16) -> bool {
    matches!(vk, 0x10 | 0x11 | 0x12 | 0xA0 | 0xA1 | 0xA2 | 0xA3 | 0xA4 | 0xA5)
}

fn is_bindable_vk(vk: u16) -> bool {
    vk != 0
        && vk < 0xFF
        && vk != VK_ESCAPE
        && vk != VK_PACKET
        && vk != VK_PROCESSKEY
        && !is_win_vk(vk)
        && !is_mouse_vk(vk)
        && !is_modifier_vk(vk)
}

impl HotkeyChord {
    pub fn from_parts(ctrl: bool, alt: bool, shift: bool, vk: u16) -> Option<Self> {
        is_bindable_vk(vk).then_some(Self { ctrl, alt, shift, vk })
    }

    /// `ctrl+shift+f9`. Modifiers are always in that order. Empty and illegal chords are `None`.
    pub fn parse(raw: &str) -> Option<Self> {
        let mut ctrl = false;
        let mut alt = false;
        let mut shift = false;
        let mut key = None;
        if raw.trim().is_empty() {
            return None;
        }
        for part in raw.split('+') {
            let part = part.trim().to_ascii_lowercase();
            if part.is_empty() {
                return None;
            }
            match part.as_str() {
                "ctrl" | "control" => {
                    if ctrl {
                        return None;
                    }
                    ctrl = true;
                }
                "alt" => {
                    if alt {
                        return None;
                    }
                    alt = true;
                }
                "shift" => {
                    if shift {
                        return None;
                    }
                    shift = true;
                }
                "win" | "lwin" | "rwin" | "meta" | "esc" | "escape" | "lbutton" | "rbutton" | "mbutton" | "xbutton1" | "xbutton2" => {
                    return None;
                }
                other => {
                    if key.is_some() {
                        return None;
                    }
                    key = Some(token_to_vk(other)?);
                }
            }
        }
        Self::from_parts(ctrl, alt, shift, key?)
    }

    pub fn to_token(self) -> String {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("ctrl".to_string());
        }
        if self.alt {
            parts.push("alt".to_string());
        }
        if self.shift {
            parts.push("shift".to_string());
        }
        parts.push(vk_name(self.vk, true));
        parts.join("+")
    }

    /// `Ctrl+Shift+F9`.
    pub fn label(self) -> String {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("Ctrl".to_string());
        }
        if self.alt {
            parts.push("Alt".to_string());
        }
        if self.shift {
            parts.push("Shift".to_string());
        }
        parts.push(vk_name(self.vk, false));
        parts.join("+")
    }
}

fn token_to_vk(token: &str) -> Option<u16> {
    if let Some((vk, _, _)) = NAMES.iter().find(|(_, name, _)| *name == token) {
        return Some(*vk);
    }
    if let Some(rest) = token.strip_prefix('f')
        && let Ok(n) = rest.parse::<u16>()
        && (1..=24).contains(&n)
    {
        return Some(0x6F + n);
    }
    if let Some(rest) = token.strip_prefix("vk")
        && let Ok(vk) = rest.parse::<u16>()
        && is_bindable_vk(vk)
    {
        return Some(vk);
    }
    let bytes = token.as_bytes();
    if bytes.len() == 1 {
        let c = bytes[0];
        if c.is_ascii_digit() || c.is_ascii_alphabetic() {
            return Some(u16::from(c.to_ascii_uppercase()));
        }
    }
    None
}

fn vk_name(vk: u16, token: bool) -> String {
    if let Some((_, tok, lab)) = NAMES.iter().find(|(code, _, _)| *code == vk) {
        return (if token { tok } else { lab }).to_string();
    }
    if (0x70..=0x87).contains(&vk) {
        return format!("{}{}", if token { 'f' } else { 'F' }, vk - 0x6F);
    }
    if (u16::from(b'0')..=u16::from(b'9')).contains(&vk) || (u16::from(b'A')..=u16::from(b'Z')).contains(&vk) {
        let c = char::from(vk as u8);
        return if token { c.to_ascii_lowercase().to_string() } else { c.to_string() };
    }
    format!("{}{vk}", if token { "vk" } else { "VK" })
}

impl Serialize for HotkeyChord {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_token())
    }
}

impl<'de> Deserialize<'de> for HotkeyChord {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Self::parse(&raw).ok_or_else(|| serde::de::Error::custom(format!("invalid hotkey: {raw}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_off_empty_and_target_only() {
        let cfg = HotkeyConfig::default();
        assert!(!cfg.enabled);
        assert!(cfg.only_when_target_focused);
        assert!(HotkeyAction::ALL.iter().all(|action| cfg.get(*action).is_none()));
        let text = toml::to_string(&cfg).unwrap();
        assert!(text.contains("enabled = false"), "{text}");
        assert!(text.contains("only_when_target_focused = true"), "{text}");
        assert!(!text.contains("start_pause"), "{text}");
        assert!(!text.contains("clear_history"), "{text}");
        let parsed: HotkeyConfig = toml::from_str(&text).unwrap();
        assert_eq!(parsed, cfg);
    }

    #[test]
    fn token_roundtrip_orders_modifiers() {
        let chord = HotkeyChord::parse("shift+f9+ctrl").unwrap();
        assert_eq!(chord.to_token(), "ctrl+shift+f9");
        assert_eq!(chord.label(), "Ctrl+Shift+F9");
        assert!(chord.ctrl && chord.shift && !chord.alt);
    }

    #[test]
    fn illegal_tokens_do_not_parse() {
        for raw in ["", "ctrl", "lwin", "win+f9", "lbutton", "esc", "ctrl+esc", "ctrl+ctrl+a", "f9+a"] {
            assert!(HotkeyChord::parse(raw).is_none(), "{raw}");
        }
    }

    #[test]
    fn assign_rejects_a_chord_another_action_owns() {
        let mut cfg = HotkeyConfig::default();
        let chord = HotkeyChord::parse("f8").unwrap();
        assert!(cfg.assign(HotkeyAction::StartPause, chord).is_ok());
        assert_eq!(cfg.assign(HotkeyAction::Stop, chord), Err(HotkeyAction::StartPause));
        assert!(cfg.stop.is_none());
        assert!(cfg.assign(HotkeyAction::StartPause, chord).is_ok());
    }

    #[test]
    fn find_uses_page_order() {
        let mut cfg = HotkeyConfig::default();
        let chord = HotkeyChord::parse("f8").unwrap();
        cfg.start_pause = Some(chord);
        cfg.stop = Some(chord);
        assert_eq!(cfg.find(chord), Some(HotkeyAction::StartPause));
    }
}
