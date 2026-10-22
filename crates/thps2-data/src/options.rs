mod keys;

pub use keys::KEY_NAMES;

use crate::fixes::Fixes;

/// A Windows profile file such as TH2_OPT.CFG, sections and keys in file order. Keys and values
/// are kept trimmed and written back as KEY=VALUE lines, the way the profile API saves them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Profile {
    sections: Vec<(String, Vec<(String, String)>)>,
}

impl Profile {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if !bytes.is_ascii() {
            return Err("options file has non ASCII bytes".into());
        }
        let mut profile = Self::default();
        for (n, line) in String::from_utf8_lossy(bytes).split("\r\n").enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(name) = line.strip_prefix('[') {
                let name = name
                    .strip_suffix(']')
                    .ok_or_else(|| format!("options line={n} unclosed section text={line:?}"))?;
                profile.sections.push((name.to_string(), Vec::new()));
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                return Err(format!("options line={n} unsupported text={line:?}"));
            };
            let Some((_, entries)) = profile.sections.last_mut() else {
                return Err(format!("options line={n} key before any section"));
            };
            entries.push((key.trim().to_string(), value.trim().to_string()));
        }
        Ok(profile)
    }

    fn entries(&self, section: &str) -> Option<&Vec<(String, String)>> {
        let found = self
            .sections
            .iter()
            .find(|s| s.0.eq_ignore_ascii_case(section));
        found.map(|s| &s.1)
    }

    pub fn get(&self, section: &str, key: &str) -> Option<&str> {
        let entries = self.entries(section)?;
        let found = entries.iter().find(|e| e.0.eq_ignore_ascii_case(key));
        found.map(|e| e.1.as_str())
    }

    /// Replaces a value where it stands, a new key goes to the end of its section and a new
    /// section to the end of the file.
    pub fn set(&mut self, section: &str, key: &str, value: &str) {
        let at = match self
            .sections
            .iter()
            .position(|s| s.0.eq_ignore_ascii_case(section))
        {
            Some(at) => at,
            None => {
                self.sections.push((section.to_string(), Vec::new()));
                self.sections.len() - 1
            }
        };
        let entries = &mut self.sections[at].1;
        match entries.iter_mut().find(|e| e.0.eq_ignore_ascii_case(key)) {
            Some(e) => e.1 = value.to_string(),
            None => entries.push((key.to_string(), value.to_string())),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut text = String::new();
        for (name, entries) in &self.sections {
            text += &format!("[{name}]\r\n");
            for (key, value) in entries {
                text += &format!("{key}={value}\r\n");
            }
        }
        text.into_bytes()
    }
}

const VERSION: &str = "6.0";
/// The one display mode the original lists.
const RESOLUTION: &str = "640x480";
const FOG_SCALES: std::ops::RangeInclusive<i32> = 50..=500;
const DEFAULT_FOG: f32 = 1.5;

/// The VIDEO section: renderer, display mode, gamma, texture quality and fog distance.
#[derive(Clone, Debug, PartialEq)]
pub struct Video {
    pub software: bool,
    /// Any RESOLUTION loads as the one mode the original lists.
    pub resolution: String,
    pub gamma: i32,
    pub high_quality: bool,
    /// Fog distance factor, kept in single precision like the original.
    pub fog: f32,
}

impl Default for Video {
    fn default() -> Self {
        Self {
            software: false,
            resolution: RESOLUTION.to_string(),
            gamma: 0,
            high_quality: false,
            fog: DEFAULT_FOG,
        }
    }
}

impl Video {
    /// The FOGSCALE the original writes: the factor times 100, cut toward zero.
    pub fn saved_fog_scale(&self) -> i32 {
        (f64::from(self.fog) * 100.0) as i32
    }

    fn load(profile: &Profile) -> Self {
        let mut v = Self::default();
        let get = |key| profile.get("VIDEO", key).unwrap_or_default();
        v.software = get("RENDERER").eq_ignore_ascii_case("Software");
        if let Some(gamma) = c_int(get("GAMMA"), true).filter(|g| (-15..=15).contains(g)) {
            v.gamma = gamma;
        }
        v.high_quality = get("QUALITY").eq_ignore_ascii_case("high");
        if let Some(scale) = c_int(get("FOGSCALE"), true).filter(|s| FOG_SCALES.contains(s)) {
            v.fog = scale as f32 * 0.01;
        }
        v
    }

    fn store(&self, profile: &mut Profile) {
        let renderer = if self.software {
            "Software"
        } else {
            "Hardware"
        };
        let quality = if self.high_quality { "High" } else { "Normal" };
        profile.set("VIDEO", "RENDERER", renderer);
        profile.set("VIDEO", "RESOLUTION", &self.resolution);
        profile.set("VIDEO", "GAMMA", &self.gamma.to_string());
        profile.set("VIDEO", "QUALITY", quality);
        profile.set("VIDEO", "FOGSCALE", &self.saved_fog_scale().to_string());
    }
}

/// Reads a number the way the C library's %d does, or %i with octal and hex prefixes.
fn c_int(text: &str, prefixes: bool) -> Option<i32> {
    let (negative, digits) = match text.trim_start().as_bytes() {
        [b'-', rest @ ..] => (true, rest),
        [b'+', rest @ ..] => (false, rest),
        rest => (false, rest),
    };
    let (radix, digits) = match digits {
        [b'0', b'x' | b'X', rest @ ..] if prefixes => (16, rest),
        [b'0', rest @ ..] if prefixes && !rest.is_empty() => (8, rest),
        rest => (10, rest),
    };
    let end = digits
        .iter()
        .position(|&d| !(d as char).is_digit(radix))
        .unwrap_or(digits.len());
    let value = match end {
        0 if radix == 10 => return None,
        0 => 0,
        _ => i64::from_str_radix(std::str::from_utf8(&digits[..end]).ok()?, radix).ok()?,
    };
    i32::try_from(if negative { -value } else { value }).ok()
}

pub const JOYPAD_ACTIONS: [&str; 10] = [
    "JUMP",
    "GRIND",
    "GRAB",
    "KICK",
    "SPINLEFT",
    "NOLLIE",
    "SPINRIGHT",
    "SWITCH",
    "PAUSE",
    "CAMERA",
];
pub const KEY_ACTIONS: [&str; 14] = [
    "JUMP",
    "GRIND",
    "GRAB",
    "KICK",
    "SPINLEFT",
    "NOLLIE",
    "SPINRIGHT",
    "SWITCH",
    "UP",
    "DOWN",
    "LEFT",
    "RIGHT",
    "PAUSE",
    "CAMERA",
];
const DEFAULT_JOYPAD: [i32; 10] = [1, 4, 2, 0, 3, 6, 5, 7, -1, -1];
const DEFAULT_KEYS: [[u8; 3]; 14] = [
    [80, 57, 28],
    [72, 47, 0],
    [75, 46, 0],
    [77, 48, 0],
    [71, 0, 0],
    [79, 0, 0],
    [73, 0, 0],
    [81, 0, 0],
    [200, 17, 0],
    [208, 31, 0],
    [203, 30, 0],
    [205, 32, 0],
    [74, 25, 1],
    [78, 24, 0],
];
const JOYPAD_BUTTONS: std::ops::RangeInclusive<i32> = -1..=32;

/// Joypad button per action, -1 for none, and up to three keys per action by scan code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Controls {
    pub joypad: [i32; 10],
    pub keys: [[u8; 3]; 14],
}

impl Default for Controls {
    fn default() -> Self {
        Self {
            joypad: DEFAULT_JOYPAD,
            keys: DEFAULT_KEYS,
        }
    }
}

fn key_code(name: &str) -> Option<u8> {
    KEY_NAMES
        .iter()
        .position(|k| k.eq_ignore_ascii_case(name))
        .map(|i| i as u8)
}

impl Controls {
    /// Any clash, unknown key or button out of range drops both sections back to the defaults.
    fn load(profile: &Profile, fixes: Fixes) -> Self {
        Self::read(profile, fixes).unwrap_or_default()
    }

    fn read(profile: &Profile, fixes: Fixes) -> Option<Self> {
        let mut c = Self::default();
        for (slot, action) in c.joypad.iter_mut().zip(JOYPAD_ACTIONS) {
            if let Some(button) = profile.get("JOYPAD", action).and_then(|t| c_int(t, false)) {
                *slot = button;
            }
        }
        let pressed = c.joypad.iter().filter(|&&b| b != -1);
        if pressed
            .clone()
            .any(|b| c.joypad.iter().filter(|&x| x == b).count() > 1)
            || c.joypad.iter().any(|b| !JOYPAD_BUTTONS.contains(b))
        {
            return None;
        }
        for (a, action) in KEY_ACTIONS.iter().enumerate() {
            let Some(text) = profile.get("KEYBOARD", action) else {
                continue;
            };
            let tokens = text.split(',').filter(|t| !t.is_empty()).take(3);
            for (slot, token) in tokens.enumerate() {
                if token.eq_ignore_ascii_case(KEY_NAMES[0]) {
                    continue;
                }
                let clash = (0..KEY_ACTIONS.len()).filter(|&o| o != a).any(|o| {
                    (0..3)
                        .filter(|&s| fixes.key_clash || s != slot)
                        .any(|s| KEY_NAMES[usize::from(c.keys[o][s])].eq_ignore_ascii_case(token))
                });
                if clash {
                    return None;
                }
                c.keys[a][slot] = key_code(token)?;
            }
        }
        Some(c)
    }

    fn store(&self, profile: &mut Profile) {
        for (button, action) in self.joypad.iter().zip(JOYPAD_ACTIONS) {
            profile.set("JOYPAD", action, &button.to_string());
        }
        for (keys, action) in self.keys.iter().zip(KEY_ACTIONS) {
            let used = keys.iter().take_while(|&&k| k != 0);
            let names: Vec<&str> = used.map(|&k| KEY_NAMES[usize::from(k)]).collect();
            profile.set("KEYBOARD", action, &names.join(","));
        }
    }
}

/// TH2_OPT.CFG read with the original's rules, and the profile it came from.
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    pub video: Video,
    pub controls: Controls,
    pub profile: Profile,
}

impl Options {
    /// Another VERSION empties the file and writes it again from the defaults. The original then
    /// writes INSTALL back with the folder it runs from, the old paths stand in for it.
    pub fn load(profile: Profile, fixes: Fixes) -> Self {
        if profile.get("VERSION", "VERSION") == Some(VERSION) {
            return Self {
                video: Video::load(&profile),
                controls: Controls::load(&profile, fixes),
                profile,
            };
        }
        let mut fresh = Self {
            video: Video::default(),
            controls: Controls::default(),
            profile: Profile::default(),
        };
        fresh.store();
        for (key, value) in profile.entries("INSTALL").into_iter().flatten() {
            fresh.profile.set("INSTALL", key, value);
        }
        fresh
    }

    /// Writes the settings back in the order the original saves them.
    pub fn store(&mut self) {
        self.profile.set("VERSION", "VERSION", VERSION);
        self.video.store(&mut self.profile);
        self.controls.store(&mut self.profile);
    }
}
