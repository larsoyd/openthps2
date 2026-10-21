use super::*;
use crate::input::{KeyEntry, is_direction};

const QUEUE: usize = 10;
const QUEUE_LIFE: u32 = 180;
const WINDOW: u16 = 0x3ff;
const NO_DIAGONALS: u16 = 0x400;
const GROUND_MOVE: u16 = 0x2000;
const NEEDS_SPECIAL: u16 = 0x8000;
const GRIND_BUTTON: u8 = 12;
const STAT_MANUAL: usize = 9;
const MANUAL_FRICTION: i32 = 20;
pub(super) const MANUAL_KICK: i32 = 4000;
const KICK_DECAY: i32 = 200;
const BALANCE_FALL: i32 = 4000;
const MANUAL_BAIL: usize = 6;
const NOSE_MANUAL_BAIL: usize = 0;
/// An animation that blocks a manual start.
const ANIM_NO_MANUAL: u16 = 0x82;

/// Manual balance: the tip angle, its rate, time in the manual, the kick a new trick adds and the
/// latch that needs up and down let go before they steer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Balance {
    pub angle: i32,
    pub rate: i32,
    pub time: i32,
    pub kick: i32,
    pub armed: bool,
}

/// An air move waiting for the running trick to finish.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueuedTrick {
    script: usize,
    queued: i32,
    pressed: i32,
}

fn sign(v: i32) -> i32 {
    if v < 0 { -1 } else { 1 }
}

impl Skater {
    /// Press recording, key matching and the queued air move ahead of the running script.
    pub(super) fn trick_input(
        &mut self,
        level: &Level,
        rules: Rules,
        pad: Pad,
        now: i32,
    ) -> Result<(), String> {
        self.presses.record_pad(pad, now);
        self.match_keys(level, rules, pad, now)?;
        self.pop_air_trick(now);
        self.run_script(level.table, level.tricks)
    }

    /// Walks the key table in order, a sequence move that misses its window or matches ends the walk.
    fn match_keys(
        &mut self,
        level: &Level,
        rules: Rules,
        pad: Pad,
        now: i32,
    ) -> Result<(), String> {
        if self.bailing {
            return Ok(());
        }
        let newest = self.presses.newest();
        for entry in &level.keys.entries {
            let ground = entry.flags & GROUND_MOVE != 0;
            // the special meter is not modelled so moves that need it never match
            if !ground && !matches!(self.state, State::Air | State::Vert)
                || entry.flags & NEEDS_SPECIAL != 0
            {
                continue;
            }
            let n = entry.buttons.len();
            let (first, last) = (entry.buttons[n - 2], entry.buttons[n - 1]);
            let window = i32::from(entry.flags & WINDOW);
            let no_diagonals = entry.flags & NO_DIAGONALS != 0;
            let p = &self.presses;
            let search = |from, skip| p.search(from, skip, window, newest, no_diagonals, now);
            let pair = n == 2 && is_direction(first) && !is_direction(last);
            let Some(p0) = search(newest, 0) else {
                if pair {
                    continue;
                }
                return Ok(());
            };
            let p1 = search(p0, 1);
            if pair {
                let held = crate::input::direction(pad);
                let Some((time, both)) = direction_move(p, first, last, p0, p1, held) else {
                    continue;
                };
                if !ground {
                    if both {
                        self.presses.consume(first);
                    }
                    self.presses.consume(last);
                }
                self.key_match(level, rules, entry, last, time, now)?;
                continue;
            }
            let Some(mut p1) = p1 else { continue };
            let strict = is_direction(first) == is_direction(last);
            let (c0, c1) = (p.code(p0), p.code(p1));
            let time = if c1 == first && c0 == last {
                p.time(p0)
            } else if !strict && c0 == first && c1 == last {
                if first == last {
                    p.time(p0)
                } else {
                    p.time(p1)
                }
            } else {
                continue;
            };
            let mut older = Vec::new();
            for &b in entry.buttons[..n - 2].iter().rev() {
                match search(p1, 1) {
                    Some(i) if p.code(i) == b => {
                        older.push(b);
                        p1 = i;
                    }
                    _ => break,
                }
            }
            if older.len() < n - 2 {
                continue;
            }
            if !ground {
                for b in [first, last].into_iter().chain(older) {
                    self.presses.consume(b);
                }
            }
            return self.key_match(level, rules, entry, last, time, now);
        }
        Ok(())
    }

    fn key_match(
        &mut self,
        level: &Level,
        rules: Rules,
        entry: &KeyEntry,
        last: u8,
        time: i32,
        now: i32,
    ) -> Result<(), String> {
        if self.air_queue.len() == QUEUE {
            return Ok(());
        }
        if last == GRIND_BUTTON {
            return Err(format!(
                "buffered grind trick script={:#x} unsupported",
                entry.script
            ));
        }
        if entry.flags & GROUND_MOVE == 0 {
            self.manual_pending = None;
            self.air_queue.push_back(QueuedTrick {
                script: entry.script,
                queued: now,
                pressed: time,
            });
        } else if self.state != State::Vert && self.manual == 0 {
            self.manual_pending = Some(entry.script);
            if self.on_ground() {
                self.start_manual(level, rules, true)?;
            }
        }
        Ok(())
    }

    /// The oldest queued air move starts once no trick runs, if it is fresh enough.
    fn pop_air_trick(&mut self, now: i32) {
        if self.trick_running || self.bailing || self.air_queue.is_empty() {
            return;
        }
        if !matches!(self.state, State::Air | State::Vert) {
            self.air_queue.clear();
            return;
        }
        let head = self.air_queue.pop_front().unwrap();
        if ((now - head.queued) as u32) < QUEUE_LIFE {
            self.presses.drop_before(head.pressed);
            self.start_script(head.script);
        }
    }

    /// A pending manual move starts its script, and a manual it sets up gets its balance.
    pub(super) fn start_manual(
        &mut self,
        level: &Level,
        rules: Rules,
        from_ground: bool,
    ) -> Result<(), String> {
        if self.bailing
            || self.anim.id == ANIM_NO_MANUAL
            || self.manual != 0
            || self.state == State::Vert
        {
            return Ok(());
        }
        if let Some(script) = self.manual_pending {
            if from_ground {
                (self.spin_angle, self.spins) = (0, 0);
            }
            self.from_ground = from_ground;
            self.manual_friction = MANUAL_FRICTION;
            self.start_script(script);
            self.run_script(level.table, level.tricks)?;
            self.manual_pending = None;
            self.air_queue.clear();
        }
        if self.manual == 0 {
            return Ok(());
        }
        let s = rules.stat(STAT_MANUAL, self.switched);
        let b = &mut self.balance;
        b.armed = false;
        b.rate = if self.manual == 1 {
            b.rate.max(s * 3000 / 10000)
        } else {
            b.rate.min(-s * 3000 / 10000)
        };
        b.rate = (((200 - s) * 5000 / 10000) * b.rate / 100) * ((b.time >> 12) + 60)
            / ((s * 6000 + 200000) / 10000);
        b.angle = (b.angle + sign(b.angle) * b.kick).abs() * sign(b.rate);
        Ok(())
    }

    /// Ground frame of a manual: kick decay, points, the tip growing with time and its own lean,
    /// up and down steering the rate and a random nudge when they rest.
    pub(super) fn balance_frame(
        &mut self,
        level: &Level,
        rules: Rules,
        pad: Pad,
        frame: FrameScale,
    ) -> Result<(), String> {
        if self.manual == 0 {
            return Ok(());
        }
        let s = rules.stat(STAT_MANUAL, self.switched);
        let points = (frame.hold_delta * self.points_rate) >> 12;
        if let Some(top) = self.panel.entries.last_mut() {
            top.points += points;
        }
        let b = &mut self.balance;
        if b.kick != 0 {
            b.kick = (b.kick - KICK_DECAY).max(0);
        }
        b.time += frame.hold_delta;
        let (t, d) = ((b.time >> 12) + 30, (s * 6000 + 200000) / 10000);
        b.angle += ((b.angle / ((s * 3300 + 280000) / 10000)) * t / d * frame.scale) >> 8;
        b.angle += ((t * b.rate / d) * frame.scale) >> 8;
        let steer = (20 * frame.scale) >> 8;
        if !pad.up && !pad.down {
            b.armed = true;
        }
        if pad.up && b.armed {
            b.rate -= steer;
        } else if pad.down && b.armed {
            b.rate += steer;
        } else if b.rate.abs() < (200 - s) * 1000 / 10000 {
            b.rate = self.rng.next((800000 - s * 3300) / 10000) + (s * 3300 - 800000) / 20000;
        } else {
            b.rate += self.rng.next(4) - 2;
        }
        let angle = self.balance.angle;
        if angle.abs() <= BALANCE_FALL {
            return Ok(());
        }
        self.balance.angle = 0;
        let over = if self.manual == 1 {
            angle > 0
        } else {
            angle < 0
        };
        if over {
            let kind = match self.rail_bail {
                0 if self.manual == 1 => MANUAL_BAIL,
                0 => NOSE_MANUAL_BAIL,
                k => k as usize,
            };
            return self.start_bail(level.table, level.tricks, kind, false);
        }
        self.end_manual()?;
        self.set_anim(level.table, ANIM_LAND)
    }

    /// Rolling round to fakie lands a manual.
    pub(super) fn manual_fakie(&mut self, table: &AnimationTable) -> Result<(), String> {
        if self.manual == 0 {
            return Ok(());
        }
        self.end_manual()?;
        self.script = None;
        self.commit_combo()?;
        self.set_anim(table, ANIM_LAND)
    }

    /// Leaving the manual drops the pending move and the presses and lands the combo.
    fn end_manual(&mut self) -> Result<(), String> {
        if self.manual == 0 {
            return Ok(());
        }
        self.manual_pending = None;
        self.presses.clear();
        self.manual = 0;
        self.commit_combo()
    }

    /// After the state handler: off the ground the manual ends, on it without one the balance resets.
    pub(super) fn settle_manual(&mut self) {
        if self.state != State::Ground {
            self.manual = 0;
        } else if self.manual == 0 {
            let b = &mut self.balance;
            (b.angle, b.rate, b.time, b.kick) = (0, 0, 0, 0);
        }
    }
}

/// A direction and a button in either order, or the held direction with the button.
fn direction_move(
    p: &crate::input::Presses,
    first: u8,
    last: u8,
    p0: usize,
    p1: Option<usize>,
    held: u8,
) -> Option<(i32, bool)> {
    let Some(p1) = p1 else {
        return (held != 0 && held == first && p.code(p0) == last).then(|| (p.time(p0), false));
    };
    let (c0, c1) = (p.code(p0), p.code(p1));
    if c0 == first && c1 == last {
        Some((p.time(p1), true))
    } else if c1 == first && c0 == last {
        Some((p.time(p0), true))
    } else if held == first && c0 == last {
        Some((p.time(p0), false))
    } else if held == first && c1 == last {
        Some((p.time(p1), false))
    } else {
        None
    }
}
