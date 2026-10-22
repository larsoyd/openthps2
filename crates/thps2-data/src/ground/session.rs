use super::*;
use crate::highscores::{Entry, HighScores};

/// Two minutes on the run clock plus the thirty ticks the clock starts short.
pub const SESSION_TICKS: i32 = 7230;
const WIND_DOWN_FRAMES: i32 = 80;
const ANIM_STEP_OFF: u16 = 0x5d;
const ANIM_BORED: u16 = 0x5f;

/// The single session clock: the limit on the run time, time up, the frames left before the run
/// ends once the skater is idle, and the end itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Session {
    pub limit: i32,
    pub time_up: bool,
    pub wind: i32,
    pub ended: bool,
    /// Past the statistics screen, on the end menu with Retry picked.
    pub menu: bool,
    jump_held: bool,
    /// The level's table, which the run's score goes into when the run ends.
    pub scores: Option<HighScores>,
    /// The row the score took, edited after the statistics screen.
    pub entered: Option<usize>,
    pub entry: Option<Entry>,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            limit: SESSION_TICKS,
            time_up: false,
            wind: 0,
            ended: false,
            menu: false,
            jump_held: false,
            scores: None,
            entered: None,
            entry: None,
        }
    }
}

impl Session {
    /// The statistics screen, the initials of a score that made the table and then the end menu
    /// each take a fresh jump press, the last one picks Retry. Other menu items are not part of
    /// the rewrite yet.
    pub(super) fn end_screens(&mut self, pad: Pad, score: i32) -> Result<bool, String> {
        let pressed = pad.jump && !self.jump_held;
        self.jump_held = pad.jump;
        if let Some(entry) = &mut self.entry {
            let scores = self
                .scores
                .as_mut()
                .ok_or("[session] initials without a table")?;
            let row = &mut scores.entries[entry.row];
            entry.edit(&mut row.initials, pad);
            if pressed {
                scores.initials = row.initials;
                (self.entry, self.menu) = (None, true);
            }
            return Ok(false);
        }
        if !pressed {
            return Ok(false);
        }
        if self.menu {
            return Ok(true);
        }
        if self.scores.is_none() {
            return Err(format!("[session] no high score table score={score}"));
        }
        match self.entered.take() {
            Some(row) => self.entry = Some(Entry::new(row)),
            None => self.menu = true,
        }
        Ok(false)
    }

    /// Time is up once the run clock passes the limit, reaching it is not enough.
    pub fn runs_out(&self, time: i32) -> bool {
        self.limit - time < 0
    }

    /// The HUD clock for a run clock reading, held at 0:00 once time is up.
    pub fn clock_text(&self, time: i32) -> String {
        let left = self.limit - time;
        let (min, sec) = if left < 0 {
            (0, 0)
        } else {
            (left / 3600, left % 3600 / 60)
        };
        format!("{min}:{sec:02}")
    }
}

impl Skater {
    /// Rolling outside a manual or stopped, the only states the run can end in.
    pub fn idle(&self) -> bool {
        (self.state == State::Ground && self.manual == 0) || self.state == State::Stopped
    }

    pub(super) fn check_time(&mut self) {
        if !self.session.time_up && self.session.runs_out(self.frame_time) {
            (self.session.time_up, self.session.wind) = (true, WIND_DOWN_FRAMES);
        }
    }

    /// Counts the idle frames after time up, waiting out points still counting into the score
    /// unless a face button banks them.
    pub(super) fn wind_down(&mut self, table: &AnimationTable, pad: Pad) -> Result<(), String> {
        if !self.idle() || !self.session.time_up {
            return Ok(());
        }
        if self.state == State::Stopped {
            match self.anim.id {
                ANIM_IDLE => self.set_anim(table, ANIM_STEP_OFF)?,
                ANIM_STEP_OFF | ANIM_BORED if self.anim.finished => {
                    self.set_anim(table, ANIM_BORED)?
                }
                _ => {}
            }
        }
        self.session.wind -= 1;
        if self.session.wind > 0 {
            return Ok(());
        }
        if self.panel.pending != 0 {
            self.session.wind = 1;
            if pad.jump || pad.grab || pad.kick || pad.grind {
                self.panel.score += self.panel.pending;
                self.panel.pending = 0;
            }
            return Ok(());
        }
        (self.session.time_up, self.session.ended) = (false, true);
        let score = self.panel.score;
        self.session.entered = self.session.scores.as_mut().and_then(|t| t.enter(score));
        Ok(())
    }

    /// After time up an idle skater only brakes, the grab button still reads.
    pub(super) fn time_up_pad(&self, pad: Pad) -> Pad {
        if !self.session.time_up || !self.idle() {
            return pad;
        }
        Pad {
            down: true,
            grab: pad.grab,
            ..Pad::default()
        }
    }
}
