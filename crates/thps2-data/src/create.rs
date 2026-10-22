use crate::rng::Rng;

/// One look a custom skater part can take, as the texture set database keeps it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TextureSet {
    id: i32,
    /// The first set of its group, or -1 for a group's first set.
    link: i32,
    part: u8,
    hidden: bool,
}

/// The create a skater texture sets of CRETEX.BIN: heads, torsos, then the parts that carry
/// only a texture, and three hidden heads for the secret skater looks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextureSets {
    sets: Vec<TextureSet>,
}

pub const CUSTOM_SKATERS: usize = 4;
pub const CUSTOM_SIZE: usize = 0x80;
pub const PARTS: usize = 7;
const MAX_SETS: usize = 0x180;
const BODY: [u8; 4] = [0x12, 5, 1, 100];
const HOME_TOWN: &[u8] = b"Woodland Hills";

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn byte(&mut self) -> Result<u8, String> {
        let b = *self
            .bytes
            .get(self.at)
            .ok_or_else(|| format!("texture sets truncated at={:#x}", self.at))?;
        self.at += 1;
        Ok(b)
    }

    fn peek(&self) -> Result<u8, String> {
        self.bytes
            .get(self.at)
            .copied()
            .ok_or_else(|| format!("texture sets truncated at={:#x}", self.at))
    }

    fn string(&mut self) -> Result<&'a [u8], String> {
        let from = self.at;
        let len = self.bytes[from.min(self.bytes.len())..]
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| format!("texture sets unterminated string at={from:#x}"))?;
        self.at += len + 1;
        Ok(&self.bytes[from..from + len])
    }
}

impl TextureSets {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let mut r = Reader { bytes, at: 0 };
        let mut sets = TextureSets { sets: Vec::new() };
        let (mut digit, mut head, mut secret_group) = (None, 0, -1);
        while r.peek()? != 0 {
            r.string()?;
            let texture = r.string()?;
            r.string()?;
            r.string()?;
            let hidden = r.byte()? != 0;
            let group = *texture
                .get(7)
                .ok_or_else(|| format!("head texture at={:#x} too short", r.at))?;
            let first = digit != Some(group);
            if first {
                (digit, head) = (Some(group), sets.next_id());
            }
            if group == b'2' {
                secret_group = head;
            }
            sets.push(if first { -1 } else { head }, 0, hidden)?;
        }
        r.at += 1;
        let mut head = sets.next_id();
        while r.peek()? != 0 {
            r.string()?;
            let variant = r.byte()? != 0;
            for _ in 0..5 {
                r.string()?;
            }
            let link = if variant { head } else { -1 };
            if !variant {
                head = sets.next_id();
            }
            sets.push(link, 1, false)?;
        }
        r.at += 1;
        while !r.string()?.is_empty() {
            r.byte()?;
        }
        while r.peek()? != 0 {
            r.string()?;
            r.string()?;
            r.string()?;
            sets.push(-1, 4, false)?;
        }
        for part in [5, 6] {
            r.at += 1;
            while r.peek()? != 0 {
                r.string()?;
                r.string()?;
                sets.push(-1, part, false)?;
            }
        }
        for _ in 0..3 {
            sets.push(secret_group, 0, true)?;
        }
        Ok(sets)
    }

    fn next_id(&self) -> i32 {
        self.sets.len() as i32
    }

    fn push(&mut self, link: i32, part: u8, hidden: bool) -> Result<(), String> {
        if self.sets.len() == MAX_SETS {
            return Err(format!("texture sets over the limit count={MAX_SETS}"));
        }
        self.sets.push(TextureSet {
            id: self.next_id(),
            link,
            part,
            hidden,
        });
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.sets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sets.is_empty()
    }

    /// The first set of a part, torsos standing in for part 2, or -1.
    pub fn first(&self, part: u8) -> i32 {
        self.sets
            .iter()
            .find(|s| s.part == part || (part == 2 && s.part == 1))
            .map_or(-1, |s| s.id)
    }

    /// Steps from the current set to the next visible one of its part whose group is the link,
    /// staying put after two laps.
    pub fn next(&self, link: i32, current: i32, step: i32) -> Result<i32, String> {
        let at = self
            .sets
            .iter()
            .position(|s| s.id == current)
            .ok_or_else(|| format!("texture set id={current} not found"))?;
        let (cur, count) = (self.sets[at], self.sets.len() as i32);
        let (mut i, mut laps) = (at as i32, 0);
        loop {
            i += step;
            if i < 0 {
                (i, laps) = (count - 1, laps + 1);
            }
            if i >= count {
                (i, laps) = (0, laps + 1);
            }
            if laps > 1 {
                return Ok(at as i32);
            }
            let s = self.sets[i as usize];
            if s.part == cur.part && (s.link == link || s.id == cur.link) && !s.hidden {
                return Ok(i);
            }
        }
    }
}

/// A custom skater slot as the boot front end leaves it: name, home town, the fixed body
/// bytes, and a random style, look and colours.
pub fn custom_defaults(
    slot: usize,
    sets: &TextureSets,
    rng: &mut Rng,
) -> Result<[u8; CUSTOM_SIZE], String> {
    if slot >= CUSTOM_SKATERS {
        return Err(format!("custom skater={slot} out of range"));
    }
    let mut b = [0; CUSTOM_SIZE];
    b[..8].copy_from_slice(format!("Skater {}", char::from(b'A' + slot as u8)).as_bytes());
    b[0x10..0x10 + HOME_TOWN.len()].copy_from_slice(HOME_TOWN);
    b[0x20..0x24].copy_from_slice(&BODY);
    b[0x25] = if slot == 0 { 0 } else { rng.next(2) as u8 };
    b[0x24] = rng.next(3) as u8;
    let mut parts: [i32; PARTS] = std::array::from_fn(|i| sets.first(i as u8));
    parts[2] = sets.first(1);
    for _ in 0..rng.next(3) {
        parts[0] = sets.next(-1, parts[0], 1)?;
    }
    let head = parts[0];
    for _ in 0..rng.next(10) {
        parts[0] = sets.next(head, parts[0], 1)?;
    }
    for (i, part) in parts.iter_mut().enumerate() {
        for _ in 0..rng.next(10) {
            if i != 0 && i != 3 {
                *part = sets.next(-1, *part, 1)?;
            }
        }
    }
    parts[2] = parts[1];
    for _ in 0..rng.next(20) {
        parts[2] = sets.next(parts[1], parts[2], 1)?;
    }
    b[0x27] = rng.next(36) as u8;
    let shade = rng.next(36) as u8;
    (b[0x28], b[0x29]) = (shade, shade);
    b[0x2a] = rng.next(36) as u8;
    for (i, part) in parts.iter().enumerate() {
        b[0x64 + 4 * i..0x68 + 4 * i].copy_from_slice(&part.to_le_bytes());
    }
    Ok(b)
}
