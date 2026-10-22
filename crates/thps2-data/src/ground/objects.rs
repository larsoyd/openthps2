use super::*;

pub(super) const SCRIPT_OBJECT: u16 = 0xcb;
pub(super) const PROP: u16 = 0x192;
const END: u16 = 0x4100;
const GOTO: u16 = 0x4101;
const GOTO_YIELD: u16 = 0x4102;
const LABEL: u16 = 0x4104;
const WAIT: u16 = 0x4280;
const WAIT_FOR_SIGNAL: u16 = 0x4281;
const PULSE: u16 = 0x42b4;
const CAREER_MESSAGE: u16 = 0x42c0;
const SOUND: u16 = 0x4507;
const SOUND_AT: u16 = 0x4508;
const SOUND_STOP: u16 = 0x4509;
const VARIABLE: u16 = 0x2000;
const RANDOM: u16 = 0x2129;

const SCAN_LABELS: u16 = 0x4105;
const DIE_QUIETLY: u16 = 0x4205;
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
    /// A kill reached it. Bounces and removal are not ported, so its place is unknown.
    pub knocked: bool,
    /// Gone from the world.
    pub dead: bool,
    signal: bool,
    active: bool,
    script: Vec<u16>,
    script_offset: usize,
    pc: usize,
    labels: [Option<usize>; 8],
    wait: u32,
    waited: u32,
    wait_from: i32,
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
            knocked: false,
            dead: false,
            signal: false,
            active: true,
            script: o.script.clone(),
            script_offset: o.script_offset,
            pc: 0,
            labels: [None; 8],
            wait: 0,
            waited: 0,
            wait_from: 0,
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

    /// A literal or a random number below the next word, none for other variables.
    fn variable(&mut self, w: u16, s: &mut Skater) -> Result<Option<i32>, String> {
        Ok(match w {
            RANDOM => {
                let n = self.word()?;
                Some(i32::from(s.rng.next(i32::from(n)) as i16))
            }
            w if w & VARIABLE != 0 => None,
            w => Some(i32::from(w as i16)),
        })
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
    fn run(&mut self, s: &mut Skater, vb: i32, signalled: bool) -> Result<(), String> {
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
                COLLISION_TYPE | SFX | SFX_AT | SOUND => {
                    self.pc += 1;
                    false
                }
                SOUND_STOP => false,
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
                DIE_QUIETLY => {
                    self.dead = true;
                    return Ok(());
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

    /// The script runs when no wait is pending. A wait that ran out ends on the next update
    /// without running it.
    pub(super) fn update(&mut self, s: &mut Skater, vb: i32) -> Result<(), String> {
        let signalled = std::mem::take(&mut self.signal);
        if self.frozen_at.is_some() || self.dead {
            return Ok(());
        }
        if self.wait != 0 {
            if self.waited >= self.wait {
                (self.wait, self.waited, self.wait_from) = (0, 0, 0);
            }
        } else if self.active {
            self.run(s, vb, signalled)?;
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
                // single session shows no career messages
                CAREER_MESSAGE => self.pc += 1,
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
