use crate::ground::Pad;

pub const ENTRIES: usize = 8;
/// Frames up or down is held before the letter starts to repeat, then every fourth frame.
const REPEAT_AFTER: i32 = 16;
const REPEAT_EVERY: i32 = 4;
pub const PULSE: i32 = 16;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HighScore {
    pub initials: [u8; 3],
    pub skater: u8,
    pub score: u32,
}

/// A level's table, the initials a new entry starts with and the skater playing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HighScores {
    pub level: usize,
    pub entries: [HighScore; ENTRIES],
    pub initials: [u8; 3],
    pub skater: u8,
}

impl HighScores {
    /// The score goes in above the first entry it reaches, shifting the rest down.
    pub fn enter(&mut self, score: i32) -> Option<usize> {
        let row = self.entries.iter().position(|e| e.score <= score as u32)?;
        self.entries.copy_within(row..ENTRIES - 1, row + 1);
        self.entries[row] = HighScore {
            initials: self.initials,
            skater: self.skater,
            score: score as u32,
        };
        Some(row)
    }
}

/// The initials of a new entry being edited: the cursor letter, the pulse a change starts and
/// the held count of up and down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    pub row: usize,
    pub cursor: usize,
    pub pulse: i32,
    held: i32,
    last: Pad,
}

impl Entry {
    pub fn new(row: usize) -> Self {
        Self {
            row,
            cursor: 0,
            pulse: 0,
            held: 0,
            last: Pad::default(),
        }
    }

    /// Up and down step the letter and repeat while held, left and right move the cursor. The
    /// pulse loses one for each row the screen draws.
    pub fn edit(&mut self, initials: &mut [u8; 3], pad: Pad) {
        self.pulse = (self.pulse - ENTRIES as i32).max(0);
        let step = self.repeat(pad);
        if step != 0 {
            self.pulse = PULSE;
            let letter = &mut initials[self.cursor];
            *letter = stepped(*letter, step);
        }
        if pad.left && !self.last.left {
            self.pulse = PULSE;
            self.cursor = (self.cursor + 2) % 3;
        }
        if pad.right && !self.last.right {
            self.pulse = PULSE;
            self.cursor = (self.cursor + 1) % 3;
        }
        self.last = pad;
    }

    /// One step on the first frame up or down is held, then again once it has been held long
    /// enough, up winning over down.
    fn repeat(&mut self, pad: Pad) -> i8 {
        if !pad.up && !pad.down {
            self.held = 0;
            return 0;
        }
        let fire = self.held == 0 || (self.held >= REPEAT_AFTER && self.held % REPEAT_EVERY == 0);
        self.held += 1;
        match (fire, pad.up) {
            (false, _) => 0,
            (true, true) => 1,
            (true, false) => -1,
        }
    }
}

/// A to Z and a space between Z and A, compared as signed chars.
fn stepped(letter: u8, step: i8) -> u8 {
    if letter == b' ' || letter == 0 {
        return if step > 0 { b'A' } else { b'Z' };
    }
    let next = (letter as i8).wrapping_add(step);
    if next > b'Z' as i8 || next < b'A' as i8 {
        b' '
    } else {
        next as u8
    }
}
