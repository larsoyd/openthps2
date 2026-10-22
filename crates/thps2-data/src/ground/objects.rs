use super::*;
use crate::collision::{Pane, model_panes};
use std::collections::{HashMap, hash_map::Entry};

pub(super) const SCRIPT_OBJECT: u16 = 0xcb;
pub(super) const PROP: u16 = 0x192;
const END: u16 = 0x4100;
const GOTO: u16 = 0x4101;
const GOTO_YIELD: u16 = 0x4102;
const LABEL: u16 = 0x4104;
const WAIT: u16 = 0x4280;
const WAIT_FOR_SIGNAL: u16 = 0x4281;
const PULSE: u16 = 0x42b4;
const GOAL_COUNTER: u16 = 0x42c0;
const SOUND: u16 = 0x4507;
const SOUND_AT: u16 = 0x4508;
const SOUND_STOP: u16 = 0x4509;
const VARIABLE: u16 = 0x2000;
const RANDOM: u16 = 0x2129;

const SCAN_LABELS: u16 = 0x4105;
const IF_EQUAL: u16 = 0x4114;
const END_IF: u16 = 0x4120;
const SHAKE: u16 = 0x4298;
const PULSE_LINKS: u16 = 0x42b2;
const REGISTER: u16 = 0x2120;
const MY_NODE: u16 = 0x212a;
/// Conditionals SkipToEndif counts as opening a level.
const CONDITIONALS: std::ops::RangeInclusive<u16> = 0x4112..=0x4116;
const DIE_QUIETLY: u16 = 0x4205;
const DIE_AND_SHATTER: u16 = 0x4301;
const SFX: u16 = 0x4290;
const SFX_AT: u16 = 0x4291;
const SPIN: u16 = 0x2127;
const SPIN_CHANGE: u16 = 0x2128;
const SET_MODEL: u16 = 0x212f;
const VELOCITY: u16 = 0x2134;
const SET_ANGLES: u16 = 0x2137;
const COLLISION_TYPE: u16 = 0x4309;
/// Operand words with this bit are negative literals, never variables.
const NEGATIVE: u16 = 0x8000;
/// Velocities inside this band stop.
const VELOCITY_REST: i32 = 0x800;
const GRAVITY: i32 = 0x8000;
/// Bounces before a bouncing prop settles.
const BOUNCES: u16 = 0x12;
const SETTLE_FRAMES: u16 = 10;
/// Horizontal speed in whole units below which a prop lands flat on its sides.
const SLOW: i32 = 0x12;
const FAST: i32 = 0x28;
const TURN_MIN: i16 = 0x28;

/// Box sizes of the object bank models by checksum, as the bouncy op reads them, and the faces
/// a prop breaks when it shatters itself.
#[derive(Clone, Debug, Default)]
pub struct PropModels {
    sizes: HashMap<u32, [u16; 3]>,
    panes: HashMap<u32, Vec<Pane>>,
}

impl PropModels {
    pub fn from_scene(scene: &crate::psx::Scene) -> Result<Self, String> {
        let colours = scene
            .chunks
            .iter()
            .find(|c| c.kind == crate::collision::COLOUR_TABLE_KIND)
            .map(|c| c.payload.as_slice());
        let (mut sizes, mut panes) = (HashMap::new(), HashMap::new());
        for m in &scene.models {
            let w = |k: usize| i16::from_le_bytes([m.header[12 + 2 * k], m.header[13 + 2 * k]]);
            let size = [0, 1, 2].map(|a| w(2 * a).wrapping_sub(w(2 * a + 1)) as u16);
            sizes.entry(m.checksum).or_insert(size);
            if let Entry::Vacant(e) = panes.entry(m.checksum) {
                e.insert(
                    model_panes(m, colours)
                        .map_err(|err| format!("prop model={:#010x} {err}", m.checksum))?,
                );
            }
        }
        Ok(Self { sizes, panes })
    }
}

/// A model of the object bank a pulse placed at its node, moved and turned by its script.
/// At an op that is not ported it stops and is no longer drawn, its place being unknown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prop {
    pub node: usize,
    pub model: Option<u32>,
    pub position: [i32; 3],
    /// Twelve bit angles.
    pub angles: [i16; 3],
    pub velocity: [i32; 3],
    pub spin: [i16; 3],
    pub spin_change: [i16; 3],
    /// The op the script stopped at, none while it runs or after its end.
    pub frozen_at: Option<u16>,
    /// Hidden from creation until its model is set, and by region scripts.
    pub hidden: bool,
    /// Stopped until a signal reaches its node.
    pub waiting: bool,
    /// Gone from the world.
    pub dead: bool,
    /// Angles a bouncing prop turns toward.
    pub targets: [i16; 3],
    pub bounces: u16,
    /// Frames left before a settled prop goes.
    pub countdown: u16,
    /// Set by the bouncy op, the sound and kick of a knock depend on it.
    kind: u16,
    size: Option<[u16; 3]>,
    bouncing: bool,
    signal: bool,
    active: bool,
    script: Vec<u16>,
    script_offset: usize,
    pc: usize,
    labels: [Option<usize>; 8],
    wait: u32,
    waited: u32,
    wait_from: i32,
    registers: [u16; 6],
    /// Set once the links have had the death pulse.
    pulsed: bool,
    /// Particles of its own shatter, for the run to take into its list.
    pub(super) particles: Vec<Particle>,
}

impl Prop {
    pub(super) fn new(node: usize, o: &crate::trig::ObjectNode) -> Self {
        Self {
            node,
            model: None,
            position: o.origin,
            angles: o.angles,
            velocity: [0; 3],
            spin: [0; 3],
            spin_change: [0; 3],
            frozen_at: None,
            hidden: true,
            waiting: false,
            dead: false,
            targets: [0; 3],
            bounces: 0,
            countdown: 0,
            kind: 0,
            size: None,
            bouncing: false,
            signal: false,
            active: true,
            script: o.script.clone(),
            script_offset: o.script_offset,
            pc: 0,
            labels: [None; 8],
            wait: 0,
            waited: 0,
            wait_from: 0,
            registers: [0; 6],
            pulsed: false,
            particles: Vec::new(),
        }
    }

    pub(super) fn signal(&mut self) {
        self.signal = true;
    }

    fn word(&mut self) -> Result<u16, String> {
        let w = self.script.get(self.pc).copied().ok_or_else(|| {
            format!(
                "prop node={} ran off its script at word={}",
                self.node, self.pc
            )
        })?;
        self.pc += 1;
        Ok(w)
    }

    /// A literal, a random number below the next word, a register or the prop's own node, none
    /// for other variables.
    fn variable(&mut self, w: u16, s: &mut Skater) -> Result<Option<i32>, String> {
        Ok(match w {
            RANDOM => {
                let n = self.word()?;
                Some(i32::from(s.rng.next(i32::from(n)) as i16))
            }
            REGISTER => Some(i32::from(self.registers[self.register()?])),
            MY_NODE => Some(self.node as i32),
            w if w & VARIABLE != 0 => None,
            w => Some(i32::from(w as i16)),
        })
    }

    /// The register index, the low byte of the next word.
    fn register(&mut self) -> Result<usize, String> {
        let n = usize::from(self.word()? & 0xff);
        if n >= self.registers.len() {
            return Err(format!("prop node={} register={n} out of range", self.node));
        }
        Ok(n)
    }

    /// Steps past the matching end if, counting the conditionals in between by their words.
    fn skip_to_end_if(&mut self) -> Result<(), String> {
        let mut depth = 0;
        loop {
            match self.word()? {
                END_IF if depth == 0 => return Ok(()),
                END_IF => depth -= 1,
                END => {
                    return Err(format!(
                        "prop node={} conditional without end if",
                        self.node
                    ));
                }
                w if CONDITIONALS.contains(&w) => depth += 1,
                _ => {}
            }
        }
    }

    fn operand(&mut self, s: &mut Skater) -> Result<Option<i32>, String> {
        match self.word()? {
            w if w & NEGATIVE != 0 => Ok(Some(i32::from(w as i16))),
            w => self.variable(w, s),
        }
    }

    fn triple(&mut self, s: &mut Skater) -> Result<Option<[i32; 3]>, String> {
        let (a, b, c) = (self.operand(s)?, self.operand(s)?, self.operand(s)?);
        Ok(a.zip(b).zip(c).map(|((a, b), c)| [a, b, c]))
    }

    fn label(&self, n: u16) -> Result<usize, String> {
        self.labels
            .get(usize::from(n))
            .copied()
            .flatten()
            .ok_or_else(|| format!("prop node={} goto label={n} unset", self.node))
    }

    /// Runs the script up to its next wait, its end or an op that is not ported.
    fn run(
        &mut self,
        s: &mut Skater,
        level: &Level,
        vb: i32,
        signalled: bool,
    ) -> Result<(), String> {
        let (models, triggers) = (level.props, level.triggers);
        loop {
            let at = self.pc;
            let op = self.word()?;
            let frozen = match op {
                END => {
                    self.active = false;
                    return Ok(());
                }
                SCAN_LABELS => false,
                LABEL => {
                    let n = self.word()?;
                    *self.labels.get_mut(usize::from(n)).ok_or_else(|| {
                        format!("prop node={} label={n} out of range", self.node)
                    })? = Some(self.pc);
                    false
                }
                GOTO | GOTO_YIELD => {
                    let n = self.word()?;
                    self.pc = self.label(n)?;
                    if op == GOTO_YIELD {
                        return Ok(());
                    }
                    false
                }
                COLLISION_TYPE => {
                    self.kind = self.word()?;
                    self.size = self.model.and_then(|m| models.sizes.get(&m).copied());
                    false
                }
                SFX | SFX_AT | SOUND => {
                    self.pc += 1;
                    false
                }
                SOUND_STOP | END_IF => false,
                REGISTER => {
                    let n = self.register()?;
                    match self.operand(s)? {
                        Some(v) => {
                            self.registers[n] = v as u16;
                            false
                        }
                        None => true,
                    }
                }
                IF_EQUAL => match (self.operand(s)?, self.operand(s)?) {
                    (Some(a), Some(b)) => {
                        if a as u16 != b as u16 {
                            self.skip_to_end_if()?;
                        }
                        false
                    }
                    _ => true,
                },
                SHAKE => match self.operand(s)? {
                    Some(v) => {
                        s.camera_shakes.push(match v {
                            0 => 2,
                            1 => 1,
                            _ => 0,
                        });
                        false
                    }
                    None => true,
                },
                PULSE_LINKS => match self.operand(s)? {
                    Some(node) => {
                        s.pulse_links(triggers, node as usize, 0)?;
                        false
                    }
                    None => true,
                },
                SET_MODEL => {
                    // the checksum sits at the next four byte boundary of the file
                    let k = ((self.script_offset + 2 * self.pc + 3) & !3) - self.script_offset;
                    let k = k / 2;
                    self.pc = k;
                    let (lo, hi) = (self.word()?, self.word()?);
                    self.model = Some(u32::from(lo) | u32::from(hi) << 16);
                    self.hidden = false;
                    false
                }
                SET_ANGLES => match self.triple(s)? {
                    Some(a) => {
                        self.angles = a.map(|v| v as i16);
                        false
                    }
                    None => true,
                },
                SPIN | SPIN_CHANGE | VELOCITY => match self.triple(s)? {
                    Some(v) if op == SPIN => {
                        self.spin = v.map(|v| v as i16);
                        false
                    }
                    Some(v) if op == SPIN_CHANGE => {
                        self.spin_change = v.map(|v| v as i16);
                        false
                    }
                    Some(v) => {
                        self.velocity = v.map(|v| i32::from(v as i16) << 12);
                        false
                    }
                    None => true,
                },
                WAIT => {
                    // a wait reads its operand as a variable whenever the variable bit is set
                    let w = self.word()?;
                    match self.variable(w, s)? {
                        Some(n) => {
                            (self.wait, self.waited, self.wait_from) = (n as u32, 0, vb);
                            return Ok(());
                        }
                        None => true,
                    }
                }
                WAIT_FOR_SIGNAL => {
                    self.waiting = !signalled;
                    if self.waiting {
                        self.pc = at;
                        return Ok(());
                    }
                    false
                }
                DIE_QUIETLY => return self.die(s, triggers),
                DIE_AND_SHATTER => {
                    self.die(s, triggers)?;
                    return self.shatter(s, level);
                }
                _ => true,
            };
            if frozen {
                self.pc = at;
                self.frozen_at = Some(op);
                return Ok(());
            }
        }
    }

    /// Dying pulses the links, once.
    fn die(&mut self, s: &mut Skater, triggers: &Triggers) -> Result<(), String> {
        if !std::mem::replace(&mut self.pulsed, true) {
            s.pulse_links(triggers, self.node, 0)?;
        }
        self.dead = true;
        Ok(())
    }

    /// Breaks every face of its model where it stands, turned like it is drawn.
    fn shatter(&mut self, s: &mut Skater, level: &Level) -> Result<(), String> {
        let panes = self
            .model
            .and_then(|m| level.props.panes.get(&m))
            .ok_or_else(|| format!("prop node={} shatters without a known model", self.node))?;
        let m = crate::pose::euler(self.angles);
        let placed: Vec<Pane> = panes.iter().map(|p| p.placed(&m, self.position)).collect();
        let (_, particles) = shatter_draws(&placed, level.world, &mut s.rng)?;
        self.particles = particles;
        Ok(())
    }

    /// The script runs when no wait is pending. A wait that ran out ends on the next update
    /// without running it.
    pub(super) fn update(&mut self, s: &mut Skater, level: &Level, vb: i32) -> Result<(), String> {
        let signalled = std::mem::take(&mut self.signal);
        if self.frozen_at.is_some() || self.dead {
            return Ok(());
        }
        if self.kind != 0 {
            return self.bounce_frame(s, level.world);
        }
        if self.wait != 0 {
            if self.waited >= self.wait {
                (self.wait, self.waited, self.wait_from) = (0, 0, 0);
            }
        } else if self.active {
            self.run(s, level, vb, signalled)?;
        }
        if self.frozen_at.is_none() && !self.dead {
            self.move_on(vb);
        }
        Ok(())
    }

    /// One step a frame, or one per two ticks of a pending wait that has not run out.
    fn move_on(&mut self, vb: i32) {
        if self.wait == 0 {
            return self.step();
        }
        while vb > self.wait_from + 2 * self.waited as i32 {
            self.waited += 1;
            if self.waited == self.wait {
                return;
            }
            self.step();
        }
    }

    fn step(&mut self) {
        for k in 0..3 {
            let v = self.velocity[k];
            self.velocity[k] = if (-VELOCITY_REST..=VELOCITY_REST).contains(&v) {
                0
            } else {
                v
            };
            self.position[k] = self.position[k].wrapping_add(self.velocity[k]);
            self.angles[k] = self.angles[k].wrapping_add(self.spin[k]) & 0xfff;
            let w = self.spin[k].wrapping_add(self.spin_change[k]);
            self.spin[k] = if (-1..=1).contains(&w) { 0 } else { w };
        }
    }

    /// A kill knocks a bouncy prop off with the skater's speed, turning it toward random angles.
    pub(super) fn knock(&mut self, s: &mut Skater, push: [i32; 3]) -> Result<(), String> {
        if self.kind == 0 || self.bouncing || self.dead {
            return Err(format!(
                "kill of prop node={} kind={} bouncing={} dead={} unsupported",
                self.node, self.kind, self.bouncing, self.dead
            ));
        }
        let speed = crate::camera::length12([push[0], 0, push[2]]);
        self.bouncing = true;
        self.bounces = 0;
        self.targets[1] = s.rng.next(0x1000) as i16;
        self.targets[0] = ((s.rng.next(2) << 11) + i32::from(self.angles[0]) - 0x400) as i16;
        self.targets[2] = ((s.rng.next(2) << 11) + i32::from(self.angles[2]) - 0x400) as i16;
        if !matches!(self.kind, 1 | 3) {
            return Err(format!(
                "prop node={} knock kind={} unsupported",
                self.node, self.kind
            ));
        }
        // the skater's grunt sound is picked at random
        s.rng.next(3);
        let up = if speed >= FAST { -0x40000 } else { -0x20000 };
        if speed < SLOW {
            self.lie_flat(s);
        }
        self.velocity = [push[0], up, push[2]];
        Ok(())
    }

    fn lie_flat(&mut self, s: &mut Skater) {
        self.targets[1] = (s.rng.next(0x400) + i32::from(self.angles[1]) - 0x200) as i16;
        self.targets[0] = self.angles[0].wrapping_add(0x200) & 0xc00;
        self.targets[2] = self.angles[2].wrapping_add(0x200) & 0xc00;
    }

    /// Turns toward the targets, falls and bounces off the world up to three times a frame.
    fn bounce_frame(&mut self, s: &mut Skater, world: &CollisionWorld) -> Result<(), String> {
        if self.countdown != 0 {
            self.countdown -= 1;
            if self.countdown == 0 {
                self.dead = true;
                return Ok(());
            }
        }
        if !self.bouncing {
            return Ok(());
        }
        self.turn();
        let size = self
            .size
            .ok_or_else(|| format!("prop node={} model={:?} box unknown", self.node, self.model))?;
        let m = crate::pose::euler(self.angles);
        let half = (0..3)
            .map(|k| i32::from(m[3 + k]).abs() * i32::from(size[k]))
            .sum::<i32>()
            >> 1;
        let mut v = self.velocity;
        v[1] += GRAVITY;
        settle_velocity(&mut v, [6, 6, 6]);
        let speed = crate::camera::length12([v[0], 0, v[2]]);
        let mut end: V3 = std::array::from_fn(|k| self.position[k].wrapping_add(v[k]));
        for _ in 0..3 {
            let hit = match world.query_filtered(self.position, end, false)? {
                Some(hit) => hit,
                None if v[1] <= 0 => return self.land(end, v),
                None => {
                    let (top, bottom) = (
                        [end[0], end[1] - half, end[2]],
                        [end[0], end[1] + half, end[2]],
                    );
                    match world.query_filtered(top, bottom, false)? {
                        Some(hit) => hit,
                        None => return self.land(end, v),
                    }
                }
            };
            if self.bounces < 5 && matches!(self.kind, 1 | 2 | 4 | 5) {
                s.rng.next(2);
            }
            self.bounces += 1;
            let n = hit.normal.map(i32::from);
            let d = dot_q12(v, n);
            for k in 0..3 {
                v[k] = v[k].wrapping_sub(mul_q12(n[k], d) << 1);
            }
            let drag = if v[1].abs() < 0xc000 {
                [2, 0, 2]
            } else {
                self.retarget(s, speed);
                [3, 2, 3]
            };
            settle_velocity(&mut v, drag);
            if self.bounces > BOUNCES {
                return self.settle();
            }
            end = std::array::from_fn(|k| hit.position_fixed[k] + mul_q12(n[k], half));
        }
        if self.bounces > BOUNCES {
            self.settle()?;
        }
        Ok(())
    }

    fn turn(&mut self) {
        self.targets = self.targets.map(|t| t & 0xfff);
        let gap = |p: &Self, k: usize| i32::from(p.targets[k]) - i32::from(p.angles[k]);
        for k in 0..3 {
            let w = (gap(self, k) >> 5) as i16;
            self.spin[k] = if (-1..=1).contains(&w) { 0 } else { w };
        }
        for k in 0..3 {
            if self.spin[k].abs() < TURN_MIN && gap(self, k).abs() > i32::from(TURN_MIN) {
                self.spin[k] = if self.spin[k] >= 0 {
                    TURN_MIN
                } else {
                    -TURN_MIN
                };
            }
        }
        for k in 0..3 {
            self.angles[k] = self.angles[k].wrapping_add(self.spin[k]) & 0xfff;
        }
    }

    fn land(&mut self, end: V3, v: V3) -> Result<(), String> {
        (self.position, self.velocity) = (end, v);
        if self.bounces > BOUNCES {
            self.settle()?;
        }
        Ok(())
    }

    /// A hard bounce picks a new heading, the first one also new tilts.
    fn retarget(&mut self, s: &mut Skater, speed: i32) {
        self.targets[1] = s.rng.next(0x1000) as i16;
        if self.bounces != 1 {
            return;
        }
        let flip = |s: &mut Skater| ((s.rng.next(2) << 11) - 0x400) as i16;
        let sideways = matches!(self.kind, 2 | 4);
        self.targets[0] = self.targets[0].wrapping_add(flip(s));
        let (x, z) = (self.targets[0], self.targets[2]);
        if sideways && ((x == 0x800 && z == 0) || (x == 0 && z == 0x800)) {
            self.targets[0] = flip(s);
        }
        self.targets[2] = z.wrapping_add(((s.rng.next(3) << 10) - 0x400) as i16);
        let (x, z) = (i32::from(self.targets[0]), i32::from(self.targets[2]));
        if sideways && ((x.abs() == 0x800 && z == 0) || (x == 0 && z.abs() == 0x800)) {
            self.targets[2] = flip(s);
        }
        if speed < SLOW {
            self.lie_flat(s);
        }
    }

    fn settle(&mut self) -> Result<(), String> {
        (self.angles[0], self.angles[2]) = (self.targets[0], self.targets[2]);
        self.countdown = SETTLE_FRAMES;
        self.bouncing = false;
        Ok(())
    }
}

/// Drags each component by its own shift, then stops what is left inside the rest band.
fn settle_velocity(v: &mut V3, shifts: [u32; 3]) {
    for k in 0..3 {
        v[k] -= v[k] >> shifts[k];
        if (-VELOCITY_REST..=VELOCITY_REST).contains(&v[k]) {
            v[k] = 0;
        }
    }
}

/// An invisible object running a node's script, the waits count down in frame time.
#[derive(Clone, Debug)]
pub struct ScriptObject {
    pub node: usize,
    script: Vec<u16>,
    pc: usize,
    labels: [Option<usize>; 8],
    wait: u32,
    active: bool,
}

impl ScriptObject {
    pub(super) fn new(node: usize, script: Vec<u16>) -> Self {
        Self {
            node,
            script,
            pc: 0,
            labels: [None; 8],
            wait: 0,
            active: true,
        }
    }

    fn word(&mut self) -> Result<u16, String> {
        let word = *self
            .script
            .get(self.pc)
            .ok_or_else(|| format!("script object node={} ran off its script", self.node))?;
        self.pc += 1;
        Ok(word)
    }

    fn value(&mut self, s: &mut Skater) -> Result<i32, String> {
        match self.word()? {
            RANDOM => {
                let n = self.word()?;
                Ok(i32::from(s.rng.next(i32::from(n)) as i16))
            }
            word if word & VARIABLE != 0 => Err(format!(
                "script object node={} variable={word:#x} unsupported",
                self.node
            )),
            word => Ok(i32::from(word)),
        }
    }

    fn label(&self, n: u16) -> Result<usize, String> {
        self.labels
            .get(usize::from(n))
            .copied()
            .flatten()
            .ok_or_else(|| format!("script object node={} goto label={n} unset", self.node))
    }

    pub(super) fn update(
        &mut self,
        s: &mut Skater,
        triggers: &Triggers,
        hold: i32,
    ) -> Result<(), String> {
        if !self.active {
            return Ok(());
        }
        if self.wait != 0 {
            self.wait = self.wait.saturating_sub(hold as u32);
            return Ok(());
        }
        loop {
            match self.word()? {
                END => {
                    self.active = false;
                    return Ok(());
                }
                LABEL => {
                    let n = self.word()?;
                    *self.labels.get_mut(usize::from(n)).ok_or_else(|| {
                        format!("script object node={} label={n} out of range", self.node)
                    })? = Some(self.pc);
                }
                GOTO => {
                    let n = self.word()?;
                    self.pc = self.label(n)?;
                }
                GOTO_YIELD => {
                    let n = self.word()?;
                    self.pc = self.label(n)?;
                    return Ok(());
                }
                PULSE => {
                    let node = self.value(s)? as usize;
                    s.pulse(triggers, self.node, node, 0)?;
                }
                WAIT => {
                    self.wait = (self.value(s)? << 12) as u32;
                    return Ok(());
                }
                GOAL_COUNTER => {
                    self.pc += 1;
                    s.career_events
                        .push(crate::career::CareerEvent::GoalCounter);
                }
                // sound is not part of the rewrite yet
                SOUND => self.pc += 1,
                SOUND_AT => self.pc += 2,
                SOUND_STOP => {}
                op => {
                    return Err(format!(
                        "script object node={} op={op:#x} unsupported",
                        self.node
                    ));
                }
            }
        }
    }
}
