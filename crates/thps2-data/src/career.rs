mod levels;

pub use levels::LEVELS;

use crate::card::{CARD_HEADER, CardIcons};
use crate::score::{Gap, HANGAR_GAPS, hangar_gap};
use crate::trig::Triggers;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GoalKind {
    Score = 0,
    Skate,
    Gaps,
    Pickups,
    Destroy,
    Trick,
    Hidden,
    Clear,
    /// Only the secret levels carry it, no check ever matches it.
    Unused,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Goal {
    pub kind: GoalKind,
    /// Points for a score goal, an object count, or the model checksum of a pickup.
    pub target: i32,
    pub item: Option<&'static str>,
    pub cash: i32,
    pub text: &'static str,
}

impl Goal {
    pub const fn new(
        kind: GoalKind,
        target: i32,
        item: Option<&'static str>,
        cash: i32,
        text: &'static str,
    ) -> Self {
        Self {
            kind,
            target,
            item,
            cash,
            text,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LevelInfo {
    pub name: &'static str,
    pub place: &'static str,
    pub competition: bool,
    pub cash_to_open: i32,
    pub medals_to_open: i32,
    /// First and last gap id of the level, compared against the id modulo 10000.
    pub gaps: [i32; 2],
    pub goals: [Goal; 10],
}

/// Levels a character keeps goal bits for, the editor park has none.
pub const PROGRESS_LEVELS: usize = 13;
pub const CHARACTERS: usize = 20;
pub const CAREER_VERSION: u32 = 0x101500;

const PROGRESS_SIZE: usize = 0x104;
const BLOCK_SIZE: usize = 0x1e00;
const SAVE_TITLE: &str = "THPS2 CAREER-";
/// Bit 15 of a competition level's goal word: every cash icon there was picked up.
const COMPETITION_CASH: u16 = 0x8000;
const MONEY_ICONS: [u8; 3] = [0x18, 0x19, 0x1a];
const LEVEL_PICKUP: u8 = 0x21;
const PICKUP_NODE: i16 = 5;

/// One character's career record as the save keeps it.
#[derive(Clone, Debug)]
pub struct Progress {
    /// Goals got over the non competition levels.
    pub tapes: u8,
    /// Career total, and what is left of it to spend.
    pub cash: i32,
    pub available: i32,
    /// One bit per goal of each level, competition medals above them.
    pub goals: [u16; PROGRESS_LEVELS],
    pub stats: [u8; 10],
    /// Cash icons picked up, 64 bits per level.
    pub money: [[u32; 2]; PROGRESS_LEVELS],
    raw: [u8; PROGRESS_SIZE],
}

/// Records are equal when they save the same bytes.
impl PartialEq for Progress {
    fn eq(&self, other: &Self) -> bool {
        self.to_bytes() == other.to_bytes()
    }
}

impl Eq for Progress {}

impl Default for Progress {
    fn default() -> Self {
        Self::read(&[0; PROGRESS_SIZE])
    }
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}

impl Progress {
    fn read(raw: &[u8; PROGRESS_SIZE]) -> Self {
        Self {
            tapes: raw[0],
            cash: u32_at(raw, 4) as i32,
            available: u32_at(raw, 8) as i32,
            goals: std::array::from_fn(|l| {
                u16::from_le_bytes([raw[0xc + 2 * l], raw[0xd + 2 * l]])
            }),
            stats: raw[0x38..0x42].try_into().unwrap(),
            money: std::array::from_fn(|l| [0, 1].map(|k| u32_at(raw, 0x44 + 8 * l + 4 * k))),
            raw: *raw,
        }
    }

    fn to_bytes(&self) -> [u8; PROGRESS_SIZE] {
        let mut b = self.raw;
        b[0] = self.tapes;
        b[4..8].copy_from_slice(&self.cash.to_le_bytes());
        b[8..12].copy_from_slice(&self.available.to_le_bytes());
        for (l, g) in self.goals.iter().enumerate() {
            b[0xc + 2 * l..0xe + 2 * l].copy_from_slice(&g.to_le_bytes());
        }
        b[0x38..0x42].copy_from_slice(&self.stats);
        for (l, words) in self.money.iter().enumerate() {
            for (k, w) in words.iter().enumerate() {
                b[0x44 + 8 * l + 4 * k..0x48 + 8 * l + 4 * k].copy_from_slice(&w.to_le_bytes());
            }
        }
        b
    }

    fn level_tapes(&self, level: usize) -> u32 {
        (self.goals[level] & 0x3ff).count_ones()
    }
}

/// A career file: the card header's name and the career block the original copies from memory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CareerSave {
    /// What follows the fixed title, as typed on the name screen.
    pub name: String,
    block: Vec<u8>,
}

impl CareerSave {
    pub fn parse(file: &[u8]) -> Result<Self, String> {
        if file.len() != CARD_HEADER + BLOCK_SIZE {
            return Err(format!("career save size={} unsupported", file.len()));
        }
        if &file[..2] != b"SC" {
            return Err(format!("career save card magic={:02x?}", &file[..2]));
        }
        let title = &file[4..0x44];
        let title = &title[..title.iter().position(|&b| b == 0).unwrap_or(title.len())];
        let title = String::from_utf8_lossy(title);
        let Some(name) = title.strip_prefix(SAVE_TITLE) else {
            return Err(format!("career save title={title:?} unsupported"));
        };
        let version = u32_at(file, CARD_HEADER);
        if version != CAREER_VERSION {
            return Err(format!("career save version={version:#x} unsupported"));
        }
        Ok(Self {
            name: name.to_string(),
            block: file[CARD_HEADER..].to_vec(),
        })
    }

    fn record(id: usize) -> Result<usize, String> {
        if id >= CHARACTERS {
            return Err(format!("career character={id} out of range"));
        }
        Ok(4 + PROGRESS_SIZE * id)
    }

    pub fn progress(&self, id: usize) -> Result<Progress, String> {
        let at = Self::record(id)?;
        Ok(Progress::read(
            self.block[at..at + PROGRESS_SIZE].try_into().unwrap(),
        ))
    }

    pub fn set_progress(&mut self, id: usize, progress: &Progress) -> Result<(), String> {
        let at = Self::record(id)?;
        self.block[at..at + PROGRESS_SIZE].copy_from_slice(&progress.to_bytes());
        Ok(())
    }

    pub fn to_bytes(&self, icons: &CardIcons) -> Result<Vec<u8>, String> {
        let mut file = icons
            .header(&format!("{SAVE_TITLE}{}", self.name), 1)?
            .to_vec();
        file.extend_from_slice(&self.block);
        Ok(file)
    }
}

/// What a skater's frame tells the career: a gap got, a combo landed with its trick names, a
/// takeoff. The original keeps waiting gaps through a takeoff in the grind or wall state, which
/// no takeoff here starts from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CareerEvent {
    Gap(i16),
    Land(Vec<String>),
    TakeOff,
    /// A script's goal counter op.
    GoalCounter,
}

const GOAL_GAP: u16 = 1;
const TRICK_GAP: u16 = 2;

/// Career state of one run on one level: the character's record and what the level offers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CareerRun {
    pub progress: Progress,
    pub level: usize,
    /// Goals got during this run, one bit each.
    pub level_goals: u16,
    /// How many goals the skater got during this run.
    pub awarded: i32,
    pub money_pickups: i32,
    pub level_pickups: i32,
    pub level_pickups_got: i32,
    /// Cash icon nodes in trigger order, a node's place is its bit in the money mask.
    pub money_nodes: Vec<usize>,
    pub gaps: i32,
    pub goal_gaps: i32,
    pub trick_gaps: i32,
    /// The goal gap and the trick gap waiting for the next landing.
    pub pending_goal_gap: Option<i16>,
    pub pending_trick_gap: Option<i16>,
    /// Goal and trick gaps counted this run, by their number among the level's gaps of that class.
    pub goal_gap_mask: u32,
    pub trick_gap_mask: u32,
    /// The gap checklist of the level, one bit per gap in table order.
    pub gap_bits: [u32; 3],
    /// Destroy goal count, 99 once the goal is given.
    pub destroyed: i32,
}

impl CareerRun {
    /// Counts the level's cash icons, pickups and gaps at the start of a run.
    pub fn new(progress: Progress, level: usize, triggers: &Triggers) -> Result<Self, String> {
        if level != 0 {
            return Err(format!("career level={level} has no gap table yet"));
        }
        let nodes = |subtypes: &[u8]| -> Vec<usize> {
            let pickup = |n: &&crate::trig::Node| {
                n.kind == PICKUP_NODE && n.raw.get(2).is_some_and(|t| subtypes.contains(t))
            };
            let all = triggers.nodes.iter().enumerate();
            all.filter(|(_, n)| pickup(n)).map(|(i, _)| i).collect()
        };
        let money_nodes = nodes(&MONEY_ICONS);
        let [first, last] = LEVELS[level].gaps;
        let active: Vec<_> = HANGAR_GAPS
            .iter()
            .filter(|g| (first..=last).contains(&(i32::from(g.id) % 10000)))
            .collect();
        let class = |c: u16| active.iter().filter(|g| g.class == c).count() as i32;
        Ok(Self {
            progress,
            level,
            level_goals: 0,
            awarded: 0,
            money_pickups: money_nodes.len() as i32,
            level_pickups: nodes(&[LEVEL_PICKUP]).len() as i32,
            level_pickups_got: 0,
            money_nodes,
            gaps: active.len() as i32,
            goal_gaps: class(1),
            trick_gaps: class(2),
            pending_goal_gap: None,
            pending_trick_gap: None,
            goal_gap_mask: 0,
            trick_gap_mask: 0,
            gap_bits: [0; 3],
            destroyed: 0,
        })
    }

    fn goals(&self) -> &'static [Goal; 10] {
        &LEVELS[self.level].goals
    }

    pub fn got(&self, goal: usize) -> bool {
        self.progress.goals[self.level] & 1 << goal != 0
    }

    /// The HUD's score pass hands out every score goal the run's counted score has reached.
    pub fn check_score(&mut self, score: i32) -> Result<(), String> {
        for n in 0..10 {
            let g = self.goals()[n];
            if g.kind == GoalKind::Score && g.target <= score && !self.got(n) {
                self.give_goal(n)?;
            }
        }
        Ok(())
    }

    pub fn give_goal(&mut self, goal: usize) -> Result<(), String> {
        if goal >= 10 || self.got(goal) {
            return Err(format!(
                "career goal={goal} level={} already got or invalid",
                self.level
            ));
        }
        self.progress.goals[self.level] |= 1 << goal;
        self.level_goals |= 1 << goal;
        let p = &self.progress;
        self.progress.tapes = (0..PROGRESS_LEVELS)
            .filter(|&l| !LEVELS[l].competition)
            .map(|l| p.level_tapes(l))
            .sum::<u32>() as u8;
        self.awarded += 1;
        let cash = self.goals()[goal].cash;
        self.progress.cash += cash;
        self.progress.available += cash;
        self.check_clear()
    }

    pub fn apply(&mut self, event: CareerEvent) -> Result<(), String> {
        match event {
            CareerEvent::Gap(id) => self.award_gap(id),
            CareerEvent::Land(names) => self.land(&names),
            CareerEvent::GoalCounter => self.count_destroyed(),
            CareerEvent::TakeOff => {
                (self.pending_goal_gap, self.pending_trick_gap) = (None, None);
                Ok(())
            }
        }
    }

    fn level_gaps(&self) -> impl Iterator<Item = &'static Gap> {
        let [first, last] = LEVELS[self.level].gaps;
        HANGAR_GAPS
            .iter()
            .filter(move |g| (first..=last).contains(&(i32::from(g.id) % 10000)))
    }

    /// Where a gap stands among the level's gaps, or among those of one class.
    fn gap_number(&self, id: i16, class: Option<u16>) -> Result<u32, String> {
        self.level_gaps()
            .filter(|g| class.is_none_or(|c| g.class == c))
            .position(|g| g.id == id)
            .map(|n| n as u32)
            .ok_or_else(|| format!("career gap id={id} not in level={}", self.level))
    }

    fn got_kind(&self, kind: GoalKind) -> bool {
        self.goal_of(kind).is_some_and(|n| self.got(n))
    }

    pub(crate) fn give_kind(&mut self, kind: GoalKind) -> Result<(), String> {
        let n = self
            .goal_of(kind)
            .ok_or_else(|| format!("career level={} has no {kind:?} goal", self.level))?;
        self.give_goal(n)
    }

    /// A got gap ticks the checklist. A new goal gap waits for the landing, a second one counts
    /// the first at once. A new trick gap waits for the landing too.
    fn award_gap(&mut self, id: i16) -> Result<(), String> {
        let n = self.gap_number(id, None)?;
        self.gap_bits[n as usize / 32] |= 1 << (n % 32);
        match hangar_gap(id)?.class {
            GOAL_GAP => {
                let bit = 1 << self.gap_number(id, Some(GOAL_GAP))?;
                if self.goal_gap_mask & bit == 0 && !self.got_kind(GoalKind::Gaps) {
                    if let Some(first) = self.pending_goal_gap {
                        self.award_goal_gap(first)?;
                    }
                    self.pending_goal_gap = Some(id);
                }
            }
            TRICK_GAP => {
                let bit = 1 << self.gap_number(id, Some(TRICK_GAP))?;
                if self.trick_gap_mask & bit == 0 && !self.got_kind(GoalKind::Trick) {
                    self.pending_trick_gap = Some(id);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn award_goal_gap(&mut self, id: i16) -> Result<(), String> {
        self.goal_gap_mask |= 1 << self.gap_number(id, Some(GOAL_GAP))?;
        if self.goal_gap_mask.count_ones() as i32 == self.goal_gaps {
            self.give_kind(GoalKind::Gaps)?;
        }
        Ok(())
    }

    /// The trick gap counts when one of the landed tricks names the goal's trick.
    fn land(&mut self, names: &[String]) -> Result<(), String> {
        if let Some(id) = self.pending_trick_gap.take() {
            let trick = self
                .goal_of(GoalKind::Trick)
                .and_then(|n| self.goals()[n].item)
                .ok_or_else(|| format!("trick gap id={id} without a trick goal"))?
                .to_ascii_lowercase();
            if names
                .iter()
                .any(|n| n.to_ascii_lowercase().contains(&trick))
            {
                self.award_trick_gap(id)?;
            }
        }
        if let Some(id) = self.pending_goal_gap.take() {
            self.award_goal_gap(id)?;
        }
        Ok(())
    }

    fn award_trick_gap(&mut self, id: i16) -> Result<(), String> {
        let bit = 1 << self.gap_number(id, Some(TRICK_GAP))?;
        if self.trick_gap_mask & bit != 0 || self.got_kind(GoalKind::Trick) {
            return Err(format!("trick gap id={id} counted twice"));
        }
        self.trick_gap_mask |= bit;
        if self.trick_gap_mask.count_ones() as i32 == self.trick_gaps {
            self.give_kind(GoalKind::Trick)?;
        }
        Ok(())
    }

    /// A cash icon: its bit in the level's money mask and the cash, then the clear check.
    pub fn give_money(&mut self, node: usize, amount: i32) -> Result<(), String> {
        let n = self
            .money_nodes
            .iter()
            .position(|&m| m == node)
            .ok_or_else(|| format!("career node={node} is not a cash icon"))?;
        self.progress.money[self.level][n / 32] |= 1 << (n % 32);
        self.progress.cash += amount;
        self.progress.available += amount;
        self.check_clear()
    }

    /// Whether a cash icon was picked up on an earlier run.
    pub fn got_money(&self, node: usize) -> bool {
        let n = self.money_nodes.iter().position(|&m| m == node);
        n.is_some_and(|n| self.progress.money[self.level][n / 32] & 1 << (n % 32) != 0)
    }

    /// One more of the level's pickups, the pickup goal once all are in.
    pub fn level_pickup(&mut self) -> Result<(), String> {
        self.level_pickups_got += 1;
        match self.goal_of(GoalKind::Pickups) {
            Some(n) if self.level_pickups_got == self.level_pickups => self.give_goal(n),
            _ => Ok(()),
        }
    }

    /// One more for the destroy goal, given at its target count, after which the count reads 99.
    fn count_destroyed(&mut self) -> Result<(), String> {
        let Some(n) = self.goal_of(GoalKind::Destroy) else {
            return Ok(());
        };
        if self.got(n) {
            return Ok(());
        }
        self.destroyed += 1;
        if self.destroyed == self.goals()[n].target {
            self.destroyed = 99;
            self.give_goal(n)?;
        }
        Ok(())
    }

    pub fn tape(&mut self) -> Result<(), String> {
        self.give_kind(GoalKind::Hidden)
    }

    fn goal_of(&self, kind: GoalKind) -> Option<usize> {
        self.goals().iter().position(|g| g.kind == kind)
    }

    fn money_got(&self) -> i32 {
        self.progress.money[self.level]
            .iter()
            .map(|w| w.count_ones() as i32)
            .sum()
    }

    /// With every cash icon taken a competition level is marked, a goal level gives its 100%
    /// goal once the other nine are got.
    fn check_clear(&mut self) -> Result<(), String> {
        if self.money_got() < self.money_pickups {
            return Ok(());
        }
        if LEVELS[self.level].competition {
            self.progress.goals[self.level] |= COMPETITION_CASH;
            return Ok(());
        }
        match self.goal_of(GoalKind::Clear) {
            Some(n) if !self.got(n) && self.progress.level_tapes(self.level) == 9 => {
                self.give_goal(n)
            }
            _ => Ok(()),
        }
    }
}
