use crate::{ground::Pad, script::TrickScripts};

mod tables;
pub use tables::{COMBOS, OP_SIZE_INDEX, SLOTS};

pub struct Slot {
    pub buttons: &'static [u8],
    pub mask: u16,
    pub rank: i16,
}

const RING: usize = 32;
/// Pad direction code by up, down, left and right bits, opposite presses folding to down or right.
const DIRECTIONS: [u8; 16] = [0, 1, 2, 2, 3, 5, 7, 7, 4, 6, 8, 8, 4, 6, 8, 8];

/// A move in the skater's key table: buttons oldest first, its trick script and its flags.
#[derive(Clone, Debug, PartialEq)]
pub struct KeyEntry {
    pub buttons: Vec<u8>,
    pub script: usize,
    pub flags: u16,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct KeyTable {
    pub entries: Vec<KeyEntry>,
}

impl KeyTable {
    /// Reads the runtime layout: count, buttons, script offset, flags, ended by a zero count.
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let word = |at: usize| {
            bytes
                .get(at..at + 2)
                .map(|w| u16::from_le_bytes([w[0], w[1]]))
                .ok_or_else(|| format!("key table truncated at={at:#x}"))
        };
        let mut entries = Vec::new();
        let mut at = 0;
        loop {
            let n = usize::from(word(at)?);
            if n == 0 {
                return Ok(Self { entries });
            }
            if !(2..=4).contains(&n) {
                return Err(format!("key table at={at:#x} buttons={n} unsupported"));
            }
            let buttons = (0..n)
                .map(|i| word(at + 2 + i * 2).map(|b| b as u8))
                .collect::<Result<_, _>>()?;
            let script = usize::from(word(at + 2 + n * 2)?);
            let flags = word(at + 4 + n * 2)?;
            entries.push(KeyEntry {
                buttons,
                script,
                flags,
            });
            at += 6 + n * 2;
        }
    }
}

/// A trick database entry: its script, the category bits its script and records give it and the
/// key flags it carries into the table.
struct Trick {
    script: usize,
    category: u16,
    flags: u16,
}

const SPECIAL: u16 = 0x8000;
const CUSTOM_SKATERS: std::ops::RangeInclusive<usize> = 13..=16;
const SPECIAL_SLOTS: usize = 5;

/// A skater's trick slots as the career record keeps them: a trick per assignable slot, then
/// five special tricks with their key combos.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrickConfig {
    pub slots: [Option<u8>; SLOT_COUNT],
    pub specials: [Option<(u8, u8)>; SPECIAL_SLOTS],
}

pub const SLOT_COUNT: usize = 0x2b;
/// Header word of the trick list custom skaters start from.
const CUSTOM_LIST: usize = 6;

impl TrickConfig {
    /// The slots a fresh career gives the skater from its trick list, specials after the rest.
    pub fn defaults(scripts: &TrickScripts, skater: usize) -> Result<Self, String> {
        Self::from_list(scripts, skater_list(scripts, skater)?)
    }

    /// The slots the custom skater default list gives.
    pub fn custom(scripts: &TrickScripts) -> Result<Self, String> {
        Self::from_list(scripts, scripts.header(CUSTOM_LIST)?)
    }

    fn from_list(scripts: &TrickScripts, list: usize) -> Result<Self, String> {
        let db = trick_database(scripts)?;
        let moves = read_list(scripts, list)?;
        let mut slots = [None; SLOT_COUNT];
        for m in moves.iter().filter(|m| m.flags & SPECIAL == 0) {
            if let Some(i) = SLOTS.iter().position(|s| s.buttons == m.buttons) {
                slots[i] = Some(trick_index(&db, m.script)?);
            }
        }
        let mut specials: [Option<(u8, u8)>; SPECIAL_SLOTS] = [None; SPECIAL_SLOTS];
        for m in moves.iter().filter(|m| m.flags & SPECIAL != 0) {
            let combo = COMBOS
                .iter()
                .position(|c| *c == m.buttons)
                .ok_or_else(|| format!("special script={:#x} has no combo", m.script))?
                as u8;
            let trick = trick_index(&db, m.script)?;
            for s in &mut specials {
                if s.is_some_and(|s| s.1 == combo) {
                    *s = None;
                }
            }
            let free = specials.iter().position(Option::is_none);
            match free {
                Some(j) if !specials.iter().flatten().any(|s| s.0 == trick) => {
                    specials[j] = Some((trick, combo))
                }
                _ => return Err(format!("special script={:#x} slot refused", m.script)),
            }
        }
        Ok(Self { slots, specials })
    }

    /// Slot bytes, then the special tricks and their combos, unused ones 0xff.
    pub fn to_bytes(&self) -> [u8; SLOT_COUNT + 2 * SPECIAL_SLOTS] {
        let mut b = [0xff; SLOT_COUNT + 2 * SPECIAL_SLOTS];
        for (i, s) in self.slots.iter().enumerate() {
            b[i] = s.unwrap_or(0xff);
        }
        for (i, s) in self.specials.iter().enumerate() {
            if let Some((trick, combo)) = s {
                b[SLOT_COUNT + i] = *trick;
                b[SLOT_COUNT + SPECIAL_SLOTS + i] = *combo;
            }
        }
        b
    }
}

/// One bit per trick database entry the skater's list holds, the bought tricks of a fresh career.
pub fn known_tricks(scripts: &TrickScripts, skater: usize) -> Result<[u32; 4], String> {
    let db = trick_database(scripts)?;
    let mut bits = [0; 4];
    for m in read_list(scripts, skater_list(scripts, skater)?)? {
        if let Some(i) = db.iter().position(|t| t.script == m.script) {
            bits[i / 32] |= 1 << (i % 32);
        }
    }
    Ok(bits)
}

fn skater_list(scripts: &TrickScripts, skater: usize) -> Result<usize, String> {
    scripts.offset(scripts.header(0)? + skater * 2)
}

fn trick_index(db: &[Trick], script: usize) -> Result<u8, String> {
    db.iter()
        .position(|t| t.script == script)
        .filter(|&i| i < 0x80)
        .map(|i| i as u8)
        .ok_or_else(|| format!("key list script={script:#x} not in the trick database"))
}

impl KeyTable {
    /// The table a skater gets from the default trick slots of a fresh career.
    pub fn build(scripts: &TrickScripts, skater: usize) -> Result<Self, String> {
        if CUSTOM_SKATERS.contains(&skater) {
            return Err(format!("custom skater={skater} trick slots unsupported"));
        }
        let db = trick_database(scripts)?;
        let config = TrickConfig::defaults(scripts, skater)?;
        let moves = read_list(scripts, skater_list(scripts, skater)?)?;
        let mut entries = Vec::new();
        let emit = |entries: &mut Vec<KeyEntry>, buttons: &[u8], t: &Trick| {
            entries.push(KeyEntry {
                buttons: buttons.to_vec(),
                script: t.script,
                flags: t.flags,
            })
        };
        let category = |entries: &mut Vec<KeyEntry>, mask: u16| -> Result<(), String> {
            for m in &moves {
                let t = &db[usize::from(trick_index(&db, m.script)?)];
                let slotted = SLOTS.iter().any(|s| s.buttons == m.buttons);
                if t.category & SPECIAL == 0 && !slotted && t.category & mask != 0 {
                    emit(entries, &m.buttons, t);
                }
            }
            for rank in 0..4 {
                for (slot, filled) in SLOTS.iter().zip(config.slots) {
                    if let Some(i) = filled
                        && slot.mask & mask != 0
                        && slot.rank == rank
                    {
                        emit(entries, slot.buttons, &db[usize::from(i)]);
                    }
                }
            }
            Ok(())
        };
        category(&mut entries, 0x1000)?;
        category(&mut entries, 0x800)?;
        for &(trick, combo) in config.specials.iter().flatten() {
            emit(
                &mut entries,
                COMBOS[usize::from(combo)],
                &db[usize::from(trick)],
            );
        }
        for mask in [0x2000, 0x4000, 0x300] {
            category(&mut entries, mask)?;
        }
        Ok(Self { entries })
    }
}

struct Move {
    buttons: Vec<u8>,
    script: usize,
    flags: u16,
}

fn read_list(scripts: &TrickScripts, mut at: usize) -> Result<Vec<Move>, String> {
    let word = |at: usize| {
        scripts
            .short(at)
            .ok_or_else(|| format!("trick list truncated at={at:#x}"))
    };
    let mut moves = Vec::new();
    loop {
        let n = word(at)?;
        if n == 0 {
            return Ok(moves);
        }
        if !(1..=4).contains(&n) {
            return Err(format!("trick list at={at:#x} buttons={n} unsupported"));
        }
        let n = n as usize;
        let buttons = (0..n)
            .map(|i| word(at + 2 + i * 2).map(|b| b as u8))
            .collect::<Result<_, _>>()?;
        moves.push(Move {
            buttons,
            script: scripts.offset(at + 2 + n * 2)?,
            flags: word(at + 4 + n * 2)? as u16,
        });
        at += 6 + n * 2;
    }
}

/// Every trick in the all tricks list once, in order of first appearance, merged by script.
fn trick_database(scripts: &TrickScripts) -> Result<Vec<Trick>, String> {
    let mut db: Vec<Trick> = Vec::new();
    for m in read_list(scripts, scripts.header(5)?)? {
        let mut category = match m.buttons.last() {
            Some(1 | 2) => 0x2000,
            Some(9) => 0x100,
            Some(10) => 0x200,
            Some(12) => 0x800,
            _ => {
                return Err(format!(
                    "all tricks script={:#x} unknown category",
                    m.script
                ));
            }
        };
        let mut pc = m.script;
        while scripts.byte(pc) != Some(7) {
            if scripts.byte(pc) == Some(0x58) {
                category |= scripts.short(pc + 1).unwrap_or(0) as u16;
            }
            pc = op_end(scripts, pc)?;
        }
        let at = match db.iter().position(|t| t.script == m.script) {
            Some(at) => {
                let t = &mut db[at];
                if m.flags & SPECIAL != 0 && t.category & SPECIAL == 0 {
                    (t.category, t.flags) = (t.category | SPECIAL, t.flags | SPECIAL);
                }
                at
            }
            None => {
                db.push(Trick {
                    script: m.script,
                    category,
                    flags: m.flags,
                });
                db.len() - 1
            }
        };
        let t = &mut db[at];
        if t.flags & 0x800 != 0 {
            t.category |= 0x800;
        }
        if t.flags & 0x4000 != 0 {
            t.category = (t.category & !0x800) | 0x4000;
        }
        t.category |= t.flags & (SPECIAL | 0x2000 | 0x1000);
    }
    Ok(db)
}

/// Where the op at pc ends, by the size classes the database scanner uses.
fn op_end(scripts: &TrickScripts, pc: usize) -> Result<usize, String> {
    let op = scripts
        .byte(pc)
        .ok_or_else(|| format!("trick scan pc={pc:#x} past the end"))?;
    let string_end = |from: usize| {
        (from..)
            .find(|&i| scripts.byte(i).is_none_or(|b| b == 0))
            .filter(|&i| scripts.byte(i).is_some())
            .map(|i| i + 1)
            .ok_or_else(|| format!("trick scan pc={pc:#x} unterminated name"))
    };
    let class = usize::from(op)
        .checked_sub(1)
        .and_then(|i| OP_SIZE_INDEX.get(i))
        .ok_or_else(|| format!("trick scan op={op:#x} pc={pc:#x} unknown"))?;
    match class {
        0 => Ok(pc + 3),
        1 => Ok(pc + 1),
        2 => string_end(pc + 1),
        3 => Ok(pc + 7),
        4 => Ok(pc + 5),
        5 => {
            let n = scripts.short(pc + 1).ok_or("trick scan truncated")?;
            Ok(pc + 3 + 2 * n as usize)
        }
        6 => Ok(pc + 4),
        7 => string_end(pc + 10),
        8 => Ok(pc + 2),
        _ => Err(format!("trick scan op={op:#x} pc={pc:#x} size unknown")),
    }
}

pub fn direction(pad: Pad) -> u8 {
    let bits = usize::from(pad.up)
        | usize::from(pad.down) << 1
        | usize::from(pad.left) << 2
        | usize::from(pad.right) << 3;
    DIRECTIONS[bits]
}

pub fn is_direction(code: u8) -> bool {
    (1..=8).contains(&code)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Press {
    code: u8,
    down: bool,
    time: i32,
}

/// The last 32 press and release edges of the pad codes the trick matcher reads.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Presses {
    ring: [Press; RING],
    next: usize,
    held: [bool; 17],
}

impl Presses {
    pub fn record(&mut self, code: u8, down: bool, time: i32) {
        let held = &mut self.held[usize::from(code)];
        if *held == down {
            return;
        }
        *held = down;
        self.ring[self.next] = Press { code, down, time };
        self.next = (self.next + 1) % RING;
    }

    /// Directions first, so a direction and a button on one frame read as direction then button.
    pub fn record_pad(&mut self, pad: Pad, time: i32) {
        let held = direction(pad);
        for code in 1..=8 {
            self.record(code, held == code, time);
        }
        for (code, down) in [
            (9, pad.grab),
            (10, pad.kick),
            (11, pad.jump),
            (12, pad.grind),
        ] {
            self.record(code, down, time);
        }
    }

    pub fn newest(&self) -> usize {
        (self.next + RING - 1) % RING
    }

    pub fn code(&self, i: usize) -> u8 {
        self.ring[i].code
    }

    pub fn time(&self, i: usize) -> i32 {
        self.ring[i].time
    }

    /// Walks back from skip records before the start to the first press, giving up at any edge
    /// older than the window or after one lap.
    pub fn search(
        &self,
        from: usize,
        skip: usize,
        window: i32,
        stop: usize,
        no_diagonals: bool,
        now: i32,
    ) -> Option<usize> {
        let mut i = (from + RING - skip) % RING;
        loop {
            let p = self.ring[i];
            if p.code != 0 && !(no_diagonals && (5..=8).contains(&p.code)) {
                if (now - p.time) as u32 > window as u32 {
                    return None;
                }
                if p.down {
                    return Some(i);
                }
            }
            i = (i + RING - 1) % RING;
            if i == stop {
                return None;
            }
        }
    }

    /// Takes the newest press of the code out of matching.
    pub fn consume(&mut self, code: u8) {
        let mut i = self.newest();
        while i != self.next {
            let p = &mut self.ring[i];
            if p.code == code && p.down {
                p.code = 0;
                return;
            }
            i = (i + RING - 1) % RING;
        }
    }

    pub fn drop_before(&mut self, time: i32) {
        for p in &mut self.ring {
            if p.down && (p.time as u32) < time as u32 {
                p.code = 0;
            }
        }
    }

    /// A release of the code newer than the time is still in the ring.
    pub fn released_after(&self, code: u8, time: i32) -> bool {
        let mut i = self.newest();
        while i != self.next {
            let p = self.ring[i];
            if p.code == code && !p.down && p.time as u32 > time as u32 {
                return true;
            }
            i = (i + RING - 1) % RING;
        }
        false
    }

    /// Distinct edge times among the live records newer than the span before now.
    pub fn edges_since(&self, now: i32, span: i32) -> i32 {
        let (mut i, mut last, mut count) = (self.newest(), 0, 0);
        while ((now - span) as u32) < self.ring[i].time as u32 && i != self.next {
            let p = self.ring[i];
            if p.time != last && p.code != 0 {
                (count, last) = (count + 1, p.time);
            }
            i = (i + RING - 1) % RING;
        }
        count
    }

    pub fn clear(&mut self) {
        for p in &mut self.ring {
            p.code = 0;
        }
    }
}
