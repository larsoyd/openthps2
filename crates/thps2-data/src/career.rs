mod fresh;
mod levels;
mod skaters;

pub use levels::LEVELS;
pub use skaters::{HIGH_SCORE_INITIALS, SKATERS};

use fresh::{HIGH_SCORE_ENTRIES, HIGH_SCORE_SIZE, HIGH_SCORES, INITIALS, TIPS_ON, TIPS_SHOWN};

use crate::card::{CARD_HEADER, CardIcons};
use crate::ground::Rules;
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
pub struct Skater {
    pub name: &'static str,
    pub stats: [u8; 10],
    pub selectable: bool,
    /// Bit 1 marks a custom skater slot, bit 5 a secret skater.
    pub flags: u32,
}

impl Skater {
    pub const fn new(name: &'static str, stats: [u8; 10], selectable: bool, flags: u32) -> Self {
        Self {
            name,
            stats,
            selectable,
            flags,
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
const GAP_BITS: usize = 0x17d4;
/// Gap checklist rows the unlock check counts, the eight career levels.
const GAP_LEVELS: usize = 8;
const UNLOCKS: usize = 0x1c24;
const TIPS_REMEMBERED: usize = TIPS_SHOWN + 8;
/// Kind of each trick tip, the first frame of a career run shows the first kind 0 tip not seen.
pub const TIP_KINDS: [u8; 22] = [
    0, 2, 2, 2, 2, 1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];
/// Tips run while no level past this one is open.
const TIP_LEVELS: usize = 3;
const CUSTOM_FLAG: u32 = 2;
/// Private Carrera, left out of the all skaters checks.
const LEFT_OUT: u32 = 0x80;
const MEDALS: u16 = 0x1c00;
const COMPETITION_LEVELS: [usize; 3] = [2, 5, 7];

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
    /// A new career's record: the stat points of the skater table and nothing got yet.
    pub fn starting(skater: &Skater) -> Self {
        Self {
            stats: skater.stats,
            ..Self::default()
        }
    }

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

    fn medals(&self) -> usize {
        (0..PROGRESS_LEVELS)
            .filter(|&l| LEVELS[l].competition && self.goals[l] & MEDALS != 0)
            .count()
    }

    /// Every tape, three golds and every competition cash icon.
    fn complete(&self) -> bool {
        self.tapes == 50
            && self.raw[3] == 3
            && COMPETITION_LEVELS
                .iter()
                .all(|&l| self.goals[l] & COMPETITION_CASH != 0)
    }
}

/// Whether the level and every one before it is open: by career cash, or by medals where the
/// table asks for medals. Chopper Drop and Skate Heaven also open by their unlock flags.
fn level_open(level: usize, progress: &Progress, unlocks: u8) -> bool {
    match level {
        8 if unlocks & 8 != 0 => return true,
        9 if unlocks & 0x10 != 0 => return true,
        _ => {}
    }
    LEVELS[..=level].iter().all(|l| {
        if l.medals_to_open == 0 {
            progress.cash >= l.cash_to_open
        } else {
            progress.medals() as i32 >= l.medals_to_open
        }
    })
}

/// The last level open among the first ten.
pub fn highest_open_level(progress: &Progress, unlocks: u8) -> usize {
    (0..10)
        .rev()
        .find(|&l| level_open(l, progress, unlocks))
        .unwrap_or(0)
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

    pub fn block(&self) -> &[u8] {
        &self.block
    }

    fn word(&self, at: usize) -> u32 {
        u32_at(&self.block, at)
    }

    fn set_word(&mut self, at: usize, value: u32) {
        self.block[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn unlocks(&self) -> u8 {
        self.block[UNLOCKS]
    }

    /// Starts a career run of the character: the record and the level's gap checklist from the
    /// save, and the first frame's trick tip.
    pub fn career_run(
        &mut self,
        skater: usize,
        level: usize,
        triggers: &Triggers,
    ) -> Result<CareerRun, String> {
        let mut run = CareerRun::new(self.progress(skater)?, level, triggers)?;
        run.opened = highest_open_level(&run.progress, self.unlocks());
        run.gap_bits = std::array::from_fn(|k| self.word(GAP_BITS + 12 * level + 4 * k));
        if run.opened <= TIP_LEVELS
            && let Some(tip) =
                (0..TIP_KINDS.len()).find(|&t| TIP_KINDS[t] == 0 && !self.tip_shown(t))
        {
            self.show_tip(tip);
        }
        Ok(run)
    }

    fn tip_shown(&self, tip: usize) -> bool {
        self.word(TIPS_SHOWN + 4 * (tip / 32)) & 1 << (tip % 32) != 0
    }

    fn show_tip(&mut self, tip: usize) {
        if self.word(TIPS_ON) == 0 {
            return;
        }
        let (bit, shown, remembered) = (
            1 << (tip % 32),
            TIPS_SHOWN + 4 * (tip / 32),
            TIPS_REMEMBERED + 4 * (tip / 32),
        );
        let before = self.word(shown);
        self.set_word(shown, before | bit);
        if self.word(remembered) & bit == 0 && before & bit != 0 {
            self.set_word(remembered, self.word(remembered) | bit);
        }
    }

    /// The end of a single player career run: record and checklist back into the block, the
    /// unlock checks and cash messages of a run without goals, then the high score table.
    pub fn end_run(&mut self, skater: usize, run: &CareerRun, score: i32) -> Result<(), String> {
        self.set_progress(skater, &run.progress)?;
        for (k, w) in run.gap_bits.iter().enumerate() {
            self.set_word(GAP_BITS + 12 * run.level + 4 * k, *w);
        }
        if run.awarded == 0 && !self.unlock(run)? {
            self.cash_messages(skater)?;
        }
        if !LEVELS[run.level].competition {
            self.enter_high_score(run.level, score, skater);
        }
        Ok(())
    }

    /// The first unlock message that applies, with the flag or record bit it sets.
    fn unlock(&mut self, run: &CareerRun) -> Result<bool, String> {
        let unlocks = self.unlocks();
        let opened = highest_open_level(&run.progress, unlocks);
        if opened != run.opened && (opened < 8 || (10..13).contains(&opened)) {
            return Ok(true);
        }
        if unlocks & 4 == 0 && run.level < 10 {
            let other = (1..GAP_LEVELS)
                .find(|&l| (0..3).any(|k| self.word(GAP_BITS + 12 * l + 4 * k) != 0));
            if let Some(l) = other {
                return Err(format!(
                    "unlock check refused: gap checklist of level={l} needs its gap count"
                ));
            }
        }
        let records: Vec<Progress> = (0..CHARACTERS)
            .map(|id| self.progress(id))
            .collect::<Result<_, _>>()?;
        for (id, p) in records.iter().enumerate() {
            if !p.complete() {
                continue;
            }
            if SKATERS[id].flags & CUSTOM_FLAG == 0 {
                if p.raw[1] & 1 == 0 {
                    let at = 4 + PROGRESS_SIZE * id + 1;
                    self.block[at] |= 1;
                    return Ok(true);
                }
            } else if unlocks & 0x20 == 0 {
                self.block[UNLOCKS] |= 0x20;
                return Ok(true);
            }
        }
        let pros = || {
            records
                .iter()
                .zip(&SKATERS)
                .filter(|(_, s)| s.flags & (CUSTOM_FLAG | LEFT_OUT) == 0)
        };
        let customs = || {
            records
                .iter()
                .zip(&SKATERS)
                .filter(|(_, s)| s.flags & CUSTOM_FLAG != 0)
        };
        let golds = customs().any(|(p, _)| p.raw[3] == 3) && pros().all(|(p, _)| p.raw[3] == 3);
        let clear = customs().any(|(p, _)| p.complete()) && pros().all(|(p, _)| p.complete());
        for (flag, met) in [(8, golds), (0x10, clear)] {
            if met && unlocks & flag == 0 {
                self.block[UNLOCKS] |= flag;
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Spending reminders once the unspent cash first reaches 500, then 2000.
    fn cash_messages(&mut self, skater: usize) -> Result<(), String> {
        let p = self.progress(skater)?;
        let unspent = p.available == p.cash;
        if self.unlocks() & 1 == 0 && p.available > 499 && unspent {
            self.block[UNLOCKS] |= 1;
        } else if self.unlocks() & 2 == 0 && p.available >= 2000 && unspent {
            self.block[UNLOCKS] |= 2;
        }
        Ok(())
    }

    /// The score goes in above the first entry it reaches, under the last initials entered.
    fn enter_high_score(&mut self, level: usize, score: i32, skater: usize) {
        let entry = |e: usize| HIGH_SCORES + HIGH_SCORE_SIZE * (HIGH_SCORE_ENTRIES * level + e);
        let Some(e) = (0..HIGH_SCORE_ENTRIES).find(|&e| self.word(entry(e) + 4) <= score as u32)
        else {
            return;
        };
        let (from, to) = (entry(e), entry(HIGH_SCORE_ENTRIES - 1));
        self.block.copy_within(from..to, from + HIGH_SCORE_SIZE);
        self.block.copy_within(INITIALS..INITIALS + 3, from);
        self.block[from + 3] = skater as u8;
        self.set_word(from + 4, score as u32);
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
    /// The score the score goals are checked against.
    Score(i32),
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
    /// The last level open when the run started.
    pub opened: usize,
    /// Kinds whose goal was got before the run, their powerups leave at once: 1 the tape, 2 the
    /// letters, 4 the level pickups.
    pub taken_kinds: u8,
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
        let opened = highest_open_level(&progress, 0);
        let mut run = Self {
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
            opened,
            taken_kinds: 0,
        };
        for (bit, kind) in [
            (1, GoalKind::Hidden),
            (2, GoalKind::Skate),
            (4, GoalKind::Pickups),
        ] {
            if run.got_kind(kind) {
                run.taken_kinds |= bit;
            }
        }
        Ok(run)
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

    /// A single player run reads the stat points of the skater's career record.
    pub fn rules(&self, base: Rules) -> Rules {
        let mut stats = base.stats;
        for (stat, &points) in stats.iter_mut().zip(&self.progress.stats) {
            *stat = i32::from(points);
        }
        Rules { stats, ..base }
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
            CareerEvent::Score(score) => self.check_score(score),
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
