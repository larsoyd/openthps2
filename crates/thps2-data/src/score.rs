/// A gap record of the executable's table: state mask, goal class, id, points and name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gap {
    pub mask: u16,
    pub class: u16,
    pub id: i16,
    pub points: u16,
    pub name: &'static str,
}

const fn gap(mask: u16, class: u16, id: i16, points: u16, name: &'static str) -> Gap {
    Gap {
        mask,
        class,
        id,
        points,
        name,
    }
}

/// Hangar's gaps in table order, ids 2700 to 2799.
pub const HANGAR_GAPS: [Gap; 21] = [
    gap(0x17, 1, 2701, 500, "[HALFPIPE HANGTIME]"),
    gap(0x19, 2, 2702, 250, "[HALFPIPE GRIND]"),
    gap(0x11, 0, 2703, 100, "[ROLLIN GAP]"),
    gap(0x13, 0, 2706, 100, "[CHOPPER HOP]"),
    gap(0x13, 1, 2709, 500, "[WINGTIP HANGTIME]"),
    gap(0x13, 1, 2710, 500, "[SKYCRANE HANGTIME]"),
    gap(0x13, 0, 2712, 250, "[FLYIN HIGH]"),
    gap(0x13, 0, 2713, 500, "[AIR OVER THE DOOR]"),
    gap(0x13, 0, 2721, 250, "[ITS COLD UP HERE]"),
    gap(0x19, 0, 2704, 250, "[LIL LIGHT HOPPER]"),
    gap(0x19, 0, 2705, 500, "[BIG LIGHT HOPPER]"),
    gap(0x19, 0, 2707, 100, "[RAIL-GUIDED MISSILE]"),
    gap(0x19, 0, 2708, 100, "[RAILDROP]"),
    gap(0x19, 0, 2714, 500, "[LIGHT CORNER]"),
    gap(0x47, 0, 2700, 500, "[INSTRUMENT LANDING]"),
    gap(0x13, 0, 2711, 100, "[HIGH STEPPIN']"),
    gap(0x13, 0, 2716, 100, "[ONE HALF PIPE LIP]"),
    gap(0x13, 0, 2717, 100, "[THE OTHER HALF PIPE LIP]"),
    gap(0x13, 0, 2718, 100, "[WIND TUNNEL BACK WALL]"),
    gap(0x13, 0, 2719, 100, "[UPWIND LIP]"),
    gap(0x13, 0, 2720, 100, "[DOWNWIND LIP]"),
];

pub fn hangar_gap(id: i16) -> Result<&'static Gap, String> {
    HANGAR_GAPS
        .iter()
        .find(|g| g.id == id)
        .ok_or_else(|| format!("gap id={id} outside the Hangar table"))
}

const SPIN_FACTOR: [i32; 6] = [0, 1, 2, 4, 6, 10];
const REPEAT_PERCENT: [i32; 5] = [100, 75, 50, 25, 10];
const HISTORY_SIZE: usize = 100;
const COUNT_UP_DELAY: i32 = 40;

/// One trick or gap of the open combo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub points: i32,
    pub spins: i32,
    /// Zero for gaps and plain entries, one closes a spin group.
    pub kind: u8,
    pub gap: Option<i16>,
    /// Tricks lose points each time the session repeats them, gaps never do.
    pub trick: bool,
    pub switched: bool,
    pub repeats: i32,
}

impl Entry {
    pub fn new(name: &str, points: i32, spins: i32, kind: u8) -> Self {
        Self {
            name: name.to_string(),
            points,
            spins,
            kind,
            gap: None,
            trick: false,
            switched: false,
            repeats: 0,
        }
    }
}

/// How the last landing lined up, judged only for combos with half spins.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Style {
    pub perfect: bool,
    pub sloppy: bool,
}

/// The combo as the trick lines show it: its entries and the landing style of the prefix.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Shown {
    pub entries: Vec<Entry>,
    pub style: Style,
}

/// What a frame did to the combo text, in order, for the panel pass after it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextEvent {
    /// A combo opened, so spins count from zero again.
    Opened,
    /// The lines were laid out again, an empty combo clears them.
    Lines(Shown),
    /// An entry joined the combo and the text grows in again.
    Restart(Shown),
    Bailed,
    Committed,
}

/// Half spins of the group holding entry i, up to and including the next kind one entry.
pub fn spin_group(e: &[Entry], i: usize) -> i32 {
    let mut start = i;
    while start > 0 && e[start - 1].kind != 1 {
        start -= 1;
    }
    let mut sum = 0;
    let mut p = start;
    while p < e.len() && e[p].kind != 1 {
        sum += e[p].spins;
        p += 1;
    }
    if p < e.len() {
        sum += e[p].spins;
    }
    sum
}

/// The landing multiplier in halves: the entry count, half a step per entry past fourteen.
pub fn multiplier(n: i32) -> i32 {
    if n < 15 { 2 * n } else { n + 14 }
}

/// Score panel: the counted score, landed points still counting up, and the open combo.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Panel {
    pub score: i32,
    pub pending: i32,
    pub delay: i32,
    pub entries: Vec<Entry>,
    /// Session counts per name and variant, the variant being the switch flag plus twice the spins.
    pub history: Vec<(String, i32, i32)>,
    /// Tricks the run has landed, shown on the statistics screen.
    pub landed: i32,
    pub best: Option<BestCombo>,
}

/// The run's highest scoring combo: its trick count, points and the trick lines listed under it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BestCombo {
    pub tricks: i32,
    pub points: i32,
    pub lines: [String; 3],
}

impl Panel {
    pub fn spins(&self) -> i32 {
        self.entries.iter().map(|e| e.spins).sum()
    }

    pub fn spin_group(&self, i: usize) -> i32 {
        spin_group(&self.entries, i)
    }

    /// Perfect landings add half, sloppy ones lose a quarter.
    pub fn sum(&self, style: Style) -> i32 {
        let mut sum: i32 = (0..self.entries.len())
            .map(|i| {
                let s = self.spin_group(i);
                let factor = if s < 6 {
                    SPIN_FACTOR[s as usize]
                } else {
                    4 * s - 10
                };
                let e = &self.entries[i];
                let repeats = if e.trick { e.repeats } else { 1 };
                e.points * (factor + 2) / 2 * REPEAT_PERCENT[(repeats - 1).clamp(0, 4) as usize]
                    / 100
            })
            .sum();
        if style.perfect {
            sum = sum * 150 / 100;
        }
        if style.sloppy {
            sum = sum * 75 / 100;
        }
        sum
    }

    pub fn total(&self, style: Style) -> i32 {
        self.sum(style) * multiplier(self.entries.len() as i32) / 2
    }

    /// The first entry of a new combo counts the last landing straight into the score.
    pub fn push(&mut self, mut entry: Entry) {
        if self.entries.is_empty() {
            self.score += self.pending;
            self.pending = 0;
            self.delay = 0;
        }
        entry.repeats = self.repeat(&entry.name, i32::from(entry.switched) + entry.spins * 2);
        self.entries.push(entry);
    }

    fn repeat(&mut self, name: &str, variant: i32) -> i32 {
        let slot = match self
            .history
            .iter()
            .position(|h| h.0 == name && h.1 == variant)
        {
            Some(slot) => slot,
            None if self.history.len() < HISTORY_SIZE => {
                self.history.push((name.to_string(), variant, 0));
                self.history.len() - 1
            }
            None => {
                self.history[0] = (name.to_string(), variant, 0);
                0
            }
        };
        self.history[slot].2 += 1;
        self.history[slot].2
    }

    pub fn commit(&mut self, style: Style) {
        if self.entries.is_empty() {
            return;
        }
        self.pending += self.total(style);
        self.entries.clear();
        self.delay = COUNT_UP_DELAY;
    }

    pub fn count_up(&mut self) {
        if self.delay > 0 {
            self.delay -= 1;
        }
        if self.delay != 0 || self.pending <= 0 {
            return;
        }
        let step = match self.pending {
            p if p > 100_000 => 2500,
            p if p > 10_000 => 1000,
            p if p > 1000 => 100,
            p if p > 25 => 25,
            p => p,
        };
        self.score += step;
        self.pending -= step;
    }
}
