use crate::{
    fixes::Fixes,
    font::Font,
    ground::{Skater, State},
    rng::Rng,
    score::{Entry, Shown, TextEvent, multiplier, spin_group},
    text::{CharMap, Effect, Prim, Text},
};

/// Scale of the trick lines and of the pop the text grows from, 20.12.
const LINE_SCALE: i32 = 0xbc0;
const POP_FROM: i32 = 0x3c0;
const POP_TO: i32 = 0x1000;
const FULL_SCALE: i32 = 0x1000;
const WIDTH_LIMIT: i32 = 480;
const LINES_AT: [i32; 2] = [256, 205];
const PENDING_AT: [i32; 2] = [256, 224];
const LINE_GAP: i32 = 5;
/// Falling glyphs stop this far below the top of the 240 line screen.
const FALL_FLOOR: i32 = 270;
const FALL_COLOUR: [u8; 3] = [0xff, 0x32, 0x32];

pub const TRICKS: Text = Text {
    hjust: 1,
    vjust: 3,
    spacing: 0,
    colour: [0x80; 3],
    shadow: None,
    fixed_pitch: false,
    scale: FULL_SCALE,
    map: CharMap::Letters,
};

/// Half spins the line shows in front of entry i, only at the start of its spin group.
fn shown_spins(entries: &[Entry], i: usize) -> i32 {
    let starts_group = i == 0 || entries[i - 1].kind == 1;
    if entries[i].kind == 2 || !starts_group {
        return 0;
    }
    spin_group(entries, i)
}

fn item(entries: &[Entry], i: usize) -> String {
    match shown_spins(entries, i) {
        0 => entries[i].name.clone(),
        spins => format!("{} {}", spins * 180, entries[i].name),
    }
}

/// The three trick lines, newest entries on the bottom one, wrapping upwards at the width limit.
pub fn lines(font: &Font, shown: &Shown) -> [String; 3] {
    let mut lines: [String; 3] = Default::default();
    let entries = &shown.entries;
    let n = entries.len();
    if n == 0 {
        return lines;
    }
    let text = Text {
        scale: LINE_SCALE,
        ..TRICKS
    };
    let prefix = if shown.style.perfect {
        Some("~Perfect]")
    } else if shown.style.sloppy {
        Some("|Sloppy]")
    } else {
        None
    };
    let (mut limit, mut cur) = (WIDTH_LIMIT, 2);
    for i in (0..n).rev() {
        let mut t = item(entries, i);
        if i != n - 1 {
            t = format!("{t} + {}", lines[cur]);
        }
        // the cut is taken again for every entry that reaches the top line
        if let Some(p) = prefix
            && (cur == 0 || i == 0)
        {
            limit -= text.width(font, &format!("{p} ... + "));
        }
        if text.width(font, &t) <= limit {
            lines[cur] = t;
        } else if cur == 0 {
            lines[0] = format!("... + {}", lines[0]);
            break;
        } else {
            cur -= 1;
            let more = if n > 1 { " +" } else { "" };
            lines[cur] = format!("{}{more}", item(entries, i));
        }
    }
    if let Some(p) = prefix {
        lines[cur] = format!("{p} {}", lines[cur]);
    }
    lines
}

/// The pop each new entry gives the lines, and the swell to red when the combo is lost.
#[derive(Clone, Debug, Default)]
struct Grow {
    active: bool,
    state: u8,
    step: i32,
    scale: i32,
    bail_from: i32,
}

impl Grow {
    fn start(&mut self) {
        (self.active, self.state, self.step) = (true, 1, 0);
    }

    fn bail(&mut self) {
        (self.state, self.bail_from, self.step) = (5, self.scale, 0);
    }

    fn update(&mut self) {
        if self.active {
            self.step += 1;
        }
    }

    /// Writes this pass's scale over the text's own while active, then moves on at the state's end.
    fn apply(&mut self, scale: &mut i32) {
        if !self.active {
            return;
        }
        let (s, line) = (self.step, LINE_SCALE);
        let (value, end) = match self.state {
            1 => ((POP_TO - POP_FROM) * s / 5 + POP_FROM, Some((5, 2))),
            2 => ((line - POP_TO) * s / 5 + POP_TO, Some((5, 3))),
            3 => (line, Some((30, 4))),
            4 => (line - s * line / 10, Some((10, 7))),
            5 => (
                (line - self.bail_from) * s / 10 + self.bail_from,
                Some((10, 6)),
            ),
            7 => (0, None),
            _ => (*scale, None),
        };
        *scale = value;
        if let Some((at, next)) = end
            && s == at
        {
            (self.state, self.step) = (next, 0);
        }
        self.scale = value;
    }

    fn done(&mut self) -> bool {
        if self.state == 6 {
            self.active = false;
        }
        !self.active
    }
}

impl Effect for Grow {
    fn colour(&mut self) -> Option<[u8; 3]> {
        if !self.active || !matches!(self.state, 5 | 6) {
            return None;
        }
        if self.state == 6 {
            self.step = 10;
        }
        let red = (0x80 + 127 * self.step / 10) as u8;
        let fade = (0x80 - 78 * self.step / 10) as u8;
        Some([red, fade, fade])
    }
}

/// A lost combo's glyphs dropping one by one off the bottom of the screen.
#[derive(Clone, Debug, Default)]
struct Fall {
    active: bool,
    timers: [i16; 32],
    blood: bool,
    index: usize,
    count: i32,
}

impl Fall {
    fn start(&mut self, rng: &mut Rng) {
        self.active = true;
        for timer in &mut self.timers {
            *timer = (rng.next(30) + 15) as i16;
            // drift and spin of the blood sprite
            rng.next(48);
            rng.next(256);
        }
        self.blood = rng.next(2) == 0;
    }

    fn update(&mut self) {
        if self.active {
            for timer in &mut self.timers {
                *timer = timer.wrapping_sub(1);
            }
        }
    }

    fn apply(&mut self, scale: &mut i32) {
        if self.active {
            (*scale, self.index, self.count) = (LINE_SCALE, 0, 0);
        }
    }

    fn done(&mut self) -> bool {
        self.active = self.count != 0;
        !self.active
    }
}

impl Effect for Fall {
    fn offset(&mut self, y: &mut i32) {
        if !self.active {
            return;
        }
        let timer = self.timers[self.index & 31];
        self.index += 1;
        if timer >= 0 {
            self.count += 1;
            return;
        }
        let d = -i32::from(timer);
        let drop = if self.blood { d << 13 } else { (d * d) << 12 };
        if *y + drop < FALL_FLOOR << 12 {
            *y += drop;
            self.count += 1;
        } else {
            *y = FALL_FLOOR << 12;
        }
    }

    fn colour(&mut self) -> Option<[u8; 3]> {
        self.active.then_some(FALL_COLOUR)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Current {
    Grow,
    Fall,
}

/// The pending points and trick lines under the skater, with the effects that animate them.
#[derive(Clone, Debug)]
pub struct TrickText {
    lines: [String; 3],
    grow: Grow,
    fall: Fall,
    current: Option<Current>,
    /// Entries went on since the last landing, so a bail turns the lines red.
    open: bool,
    bailed: bool,
    /// The multiplied combo as last shown, what a bail leaves on screen.
    lost: i32,
    /// Half spins the lines last restarted at.
    mark: i32,
    /// Scale of the shared text object, kept between passes like the original's.
    scale: i32,
}

impl Default for TrickText {
    fn default() -> Self {
        Self {
            lines: Default::default(),
            grow: Grow::default(),
            fall: Fall::default(),
            current: None,
            open: false,
            bailed: false,
            lost: 0,
            mark: 0,
            scale: FULL_SCALE,
        }
    }
}

impl TrickText {
    /// One panel pass after a frame: the frame's text events, the effect steps, then the pending
    /// points and the lines in submission order. The lost combo's fall draws random numbers.
    pub fn pass(
        &mut self,
        font: &Font,
        skater: &mut Skater,
        fixes: Fixes,
    ) -> Result<Vec<Prim>, String> {
        for event in &skater.text_events {
            self.apply(font, event);
        }
        if skater.session.ended {
            return Ok(Vec::new());
        }
        self.fall.update();
        self.grow.update();
        let mut prims = self.pending(font, skater, fixes)?;
        if !skater.panel.entries.is_empty() && skater.spins > self.mark {
            self.restart(font, &skater.shown());
            self.mark = skater.spins;
        }
        prims.extend(self.trick_lines(font, &mut skater.rng)?);
        Ok(prims)
    }

    fn apply(&mut self, font: &Font, event: &TextEvent) {
        match event {
            TextEvent::Opened => self.mark = 0,
            TextEvent::Lines(shown) => self.lines = lines(font, shown),
            TextEvent::Restart(shown) => self.restart(font, shown),
            TextEvent::Bailed => {
                if self.open {
                    self.open = false;
                    if !self.grow.done() {
                        self.grow.bail();
                        self.bailed = true;
                    }
                }
            }
            TextEvent::Committed => (self.open, self.bailed) = (false, false),
        }
    }

    fn restart(&mut self, font: &Font, shown: &Shown) {
        (self.open, self.fall.active) = (true, false);
        if !shown.entries.is_empty() {
            self.grow.start();
            self.current = Some(Current::Grow);
            self.lines = lines(font, shown);
        }
    }

    /// Combo points with the multiplier while the combo runs, then the landed points counting
    /// into the score, or the lost total falling with the lines.
    fn pending(&mut self, font: &Font, skater: &Skater, fixes: Fixes) -> Result<Vec<Prim>, String> {
        let panel = &skater.panel;
        let n = panel.entries.len() as i32;
        let (mut halves, mut points) = (0, 0);
        if n > 0 {
            halves = if fixes.multiplier_text {
                multiplier(n)
            } else if 2 * n < 15 {
                2 * n
            } else {
                n + 7
            };
            if skater.state == State::Vert && panel.spins() == 1 {
                halves -= 1;
            }
            points = panel.sum(skater.landed_style());
        }
        if panel.pending != 0 {
            points = panel.pending;
        }
        let frozen = self.bailed || self.current == Some(Current::Fall);
        let mut prims = Vec::new();
        if points != 0 || frozen {
            let value = if frozen { self.lost } else { points };
            let hjust = if halves > 2 { 2 } else { 1 };
            prims.extend(self.pending_text(font, hjust, &value.to_string())?);
            if halves > 2 {
                let times = if halves % 2 != 0 {
                    format!(" x{}.5", halves >> 1)
                } else {
                    format!(" x{}", halves >> 1)
                };
                prims.extend(self.pending_text(font, 0, &times)?);
            }
            if points != 0 {
                self.lost = (halves.wrapping_mul(points) as u32 >> 1) as i32;
            }
        }
        Ok(prims)
    }

    /// The pending text falls with the lines but always at full size.
    fn pending_text(&mut self, font: &Font, hjust: u8, s: &str) -> Result<Vec<Prim>, String> {
        self.fall.apply(&mut self.scale);
        self.scale = FULL_SCALE;
        let text = Text {
            hjust,
            scale: self.scale,
            ..TRICKS
        };
        let [x, y] = PENDING_AT;
        text.draw_with(font, x, y, s, &mut self.fall)
    }

    /// Lines two and one upwards from the bottom, then line zero, through the current effect.
    fn trick_lines(&mut self, font: &Font, rng: &mut Rng) -> Result<Vec<Prim>, String> {
        let Some(current) = self.current else {
            return Ok(Vec::new());
        };
        let effect: &mut dyn Effect = match current {
            Current::Grow => {
                self.grow.apply(&mut self.scale);
                &mut self.grow
            }
            Current::Fall => {
                self.fall.apply(&mut self.scale);
                &mut self.fall
            }
        };
        let text = Text {
            scale: self.scale,
            ..TRICKS
        };
        let [x, mut y] = LINES_AT;
        let mut prims = Vec::new();
        for line in [&self.lines[2], &self.lines[1]] {
            if !line.is_empty() {
                prims.extend(text.draw_with(font, x, y, line, effect)?);
                y -= text.height(font, line) + LINE_GAP;
            }
        }
        prims.extend(text.draw_with(font, x, y, &self.lines[0], effect)?);
        match current {
            Current::Grow => {
                if self.grow.done() && self.bailed {
                    self.fall.start(rng);
                    self.bailed = false;
                    self.current = Some(Current::Fall);
                }
            }
            Current::Fall => {
                if self.fall.done() {
                    self.current = None;
                }
            }
        }
        Ok(prims)
    }
}
