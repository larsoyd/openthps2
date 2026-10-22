use super::*;

pub(super) const PICKUP_NODE: i16 = 5;
/// Letter pickups in panel order, S K A T E.
pub const LETTERS: [u8; 5] = [5, 4, 6, 15, 10];
const TAPE: u8 = 0x10;
const CASH: [(u8, i32); 3] = [(0x18, 250), (0x19, 50), (0x1a, 100)];
const LEVEL_PICKUP: u8 = 0x21;
/// The timer of a pickup that waits to be taken, any other value counts down to its removal.
const ACTIVE: u16 = 0xffff;
const BOB_HEIGHT: i32 = 50;
const REACH: u16 = 300;
const TAPE_REACH: u16 = 500;
/// Past this rough distance the object walk suspends a powerup until the skater comes back.
const SUSPEND_DISTANCE: i32 = 13000;

/// A powerup a trigger node places: a letter, the secret tape, a cash icon or a level pickup.
#[derive(Clone, Debug)]
pub struct Pickup {
    pub node: usize,
    pub kind: u8,
    base: V3,
    pub position: V3,
    pub phase: u16,
    /// Bob phase step per design frame.
    pub speed: u16,
    pub reach: u16,
    pub timer: u16,
    /// Hidden by a region script, which stops the bob but not the touch.
    pub hidden: bool,
    /// The shadow comes with the powerup and is dropped for good the first time it is out of view.
    pub shadow: bool,
    pub glow: bool,
}

fn word(raw: &[u8], at: usize) -> Result<i32, String> {
    raw.get(at..at + 4)
        .map(|b| i32::from_le_bytes(b.try_into().unwrap()))
        .ok_or_else(|| format!("pickup node truncated at={at}"))
}

impl Pickup {
    /// Outside a career run, for cash an earlier run took and for a kind whose goal is got, the
    /// pickup only lives one frame.
    pub fn new(
        node: usize,
        triggers: &Triggers,
        rng: &mut Rng,
        career: Option<&CareerRun>,
    ) -> Result<Self, String> {
        let n = &triggers.nodes[node];
        let kind = n.raw[2];
        let links = usize::from(u16::from_le_bytes([n.raw[4], n.raw[5]]));
        if links != 0 {
            return Err(format!("pickup node={node} with links unsupported"));
        }
        let at = ((n.offset + 6 + 2 * links + 3) & !3) - n.offset;
        let base = [
            word(&n.raw, at)?,
            word(&n.raw, at + 4)?,
            word(&n.raw, at + 8)?,
        ]
        .map(|c| c << 12);
        let record = |k: usize| u16::from_le_bytes([n.raw[at + 12 + k], n.raw[at + 13 + k]]);
        if record(2) == 0 {
            return Err(format!(
                "pickup node={node} dropping to the ground unsupported"
            ));
        }
        let speed = 90 + rng.next(20) as u16;
        let kind_bit = match kind {
            TAPE => 1,
            LEVEL_PICKUP => 4,
            _ if LETTERS.contains(&kind) => 2,
            _ => 0,
        };
        let taken = career.is_none_or(|c| {
            (CASH.iter().any(|m| m.0 == kind) && c.got_money(node)) || c.taken_kinds & kind_bit != 0
        });
        let known = LETTERS.contains(&kind)
            || [TAPE, LEVEL_PICKUP].contains(&kind)
            || CASH.iter().any(|m| m.0 == kind);
        if !known {
            return Err(format!("pickup node={node} kind={kind} unsupported"));
        }
        Ok(Self {
            node,
            kind,
            base,
            position: base,
            phase: 0,
            speed,
            reach: if kind == TAPE { TAPE_REACH } else { REACH },
            timer: if taken { 1 } else { record(4) },
            hidden: false,
            shadow: true,
            glow: false,
        })
    }

    pub fn active(&self) -> bool {
        self.timer == ACTIVE
    }

    /// Counts the timer down and bobs above the base point while the skater is near. False once
    /// the pickup is gone.
    pub fn update(&mut self, skater: V3, scale: i32) -> bool {
        let distance = rough_distance(skater, self.position);
        // a suspended powerup loses its glow and its shadow for good and runs nothing
        if distance > SUSPEND_DISTANCE {
            (self.shadow, self.glow) = (false, false);
            return true;
        }
        if self.timer != ACTIVE {
            self.timer = self.timer.saturating_sub(1);
            if self.timer == 0 {
                return false;
            }
        }
        // hidden or past the far clip a powerup stops bobbing
        if self.hidden {
            return true;
        }
        if distance > crate::render::HANGAR_VIEWPORT.far {
            (self.shadow, self.glow) = (false, false);
            return true;
        }
        self.glow = true;
        let lift = crate::camera::rsin(i32::from(self.phase & 0xfff)) * BOB_HEIGHT;
        self.position = self.base;
        self.position[1] += lift / 4096 * 4096;
        self.phase = self
            .phase
            .wrapping_add(((i32::from(self.speed) * scale) >> 8) as u16);
        true
    }
}

/// Largest axis plus half the middle and a quarter of the smallest, in whole units.
fn rough_distance(a: V3, b: V3) -> i32 {
    let mut d: V3 = std::array::from_fn(|i| (a[i] - b[i]).abs());
    d.sort_unstable();
    (d[2] + (d[1] >> 1) + (d[0] >> 2)) >> 12
}

fn to_short(v: i32) -> i32 {
    i32::from(v as i16)
}

fn squared(v: V3) -> i32 {
    v.iter()
        .fold(0i32, |s, &c| s.wrapping_add(c.wrapping_mul(c)))
}

/// The pickup the frame's travel passes nearest along, by the native segment and sphere test.
pub(super) fn swept(from: V3, to: V3, pickups: &[Pickup]) -> Option<usize> {
    let d: V3 = std::array::from_fn(|a| to_short((to[a] - from[a]) >> 12));
    let length = sqrt_int(squared(d));
    if length == 0 {
        return None;
    }
    let dir: V3 = d.map(|c| to_short((c << 12) / length));
    let lo: V3 = std::array::from_fn(|a| from[a].min(to[a]));
    let hi: V3 = std::array::from_fn(|a| from[a].max(to[a]));
    let mut best = (i32::MAX, None);
    for (i, p) in pickups.iter().enumerate() {
        let r = i32::from(p.reach) << 12;
        if (0..3).any(|a| p.position[a] + r < lo[a] || p.position[a] - r > hi[a]) {
            continue;
        }
        let rel: V3 = std::array::from_fn(|a| to_short((p.position[a] - from[a]) >> 12));
        let across = [
            (dir[1] * rel[2] - dir[2] * rel[1]) >> 12,
            (dir[2] * rel[0] - dir[0] * rel[2]) >> 12,
            (dir[0] * rel[1] - dir[1] * rel[0]) >> 12,
        ]
        .map(|c| c.clamp(-0x8000, 0x7fff));
        let reach = (r >> 12) * (r >> 12);
        if squared(across) >= reach {
            continue;
        }
        let t = (dir[0] * rel[0] + dir[1] * rel[1] + dir[2] * rel[2]) >> 12;
        if t >= best.0 {
            continue;
        }
        let end: V3 = std::array::from_fn(|a| to_short((p.position[a] - to[a]) >> 12));
        let outside = (t < 0 && squared(rel) >= reach) || (t > length && squared(end) >= reach);
        if !outside {
            best = (t, Some(i));
        }
    }
    best.1
}

impl Skater {
    /// A touched pickup gives its career award, a cash icon or level pickup met on the ground
    /// wrapped in a takeoff and landing. Hidden ones, and cash and level pickups during a bail,
    /// stay where they are.
    pub(super) fn take_pickup(
        &mut self,
        career: &mut CareerRun,
        p: &Pickup,
    ) -> Result<bool, String> {
        if p.hidden {
            return Ok(false);
        }
        if let Some(slot) = LETTERS.iter().position(|&k| k == p.kind) {
            self.letters[slot] = 1;
            if self.letters.iter().all(|&l| l == 1) {
                career.give_kind(GoalKind::Skate)?;
            }
            return Ok(true);
        }
        if p.kind == TAPE {
            career.tape()?;
            return Ok(true);
        }
        if self.bailing {
            return Ok(false);
        }
        let rolling = self.state == State::Ground;
        if rolling {
            self.open_combo();
        }
        match CASH.iter().find(|m| m.0 == p.kind) {
            Some(&(_, amount)) => career.give_money(p.node, amount)?,
            None => career.level_pickup()?,
        }
        if rolling {
            (self.spins, self.spin_angle) = (0, 0);
            self.commit_combo()?;
        }
        Ok(true)
    }
}
