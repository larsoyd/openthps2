use super::*;

pub(super) const SCRIPT_OBJECT: u16 = 0xcb;
pub(super) const PROP: u16 = 0x192;
const END: u16 = 0x4100;
const GOTO: u16 = 0x4101;
const GOTO_YIELD: u16 = 0x4102;
const LABEL: u16 = 0x4104;
const WAIT: u16 = 0x4280;
const PULSE: u16 = 0x42b4;
const CAREER_MESSAGE: u16 = 0x42c0;
const SOUND: u16 = 0x4507;
const SOUND_AT: u16 = 0x4508;
const SOUND_STOP: u16 = 0x4509;
const VARIABLE: u16 = 0x2000;
const RANDOM: u16 = 0x2129;

const SCAN_LABELS: u16 = 0x4105;
const SET_MODEL: u16 = 0x212f;
const SET_ANGLES: u16 = 0x2137;
const COLLISION_TYPE: u16 = 0x4309;

/// A model of the object bank a pulse placed at its node. Its script starts on its first update and
/// runs through waits and the statements that only place it. The ops that move it are not ported:
/// a prop stops at the first one and from then on its place is unknown, so it is not drawn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prop {
    pub node: usize,
    pub model: Option<u32>,
    pub position: [i32; 3],
    pub angles: [i16; 3],
    /// The op the script stopped at, none while it runs or after its end.
    pub frozen_at: Option<u16>,
    /// Hidden from creation until its model is set, and by region scripts.
    pub hidden: bool,
    started: bool,
    script: Vec<u16>,
    script_offset: usize,
    pc: usize,
    wait: u32,
}

impl Prop {
    pub(super) fn new(node: usize, o: &crate::trig::ObjectNode) -> Self {
        Self {
            node,
            model: None,
            position: o.origin,
            angles: o.angles,
            frozen_at: None,
            hidden: true,
            started: false,
            script: o.script.clone(),
            script_offset: o.script_offset,
            pc: 0,
            wait: 0,
        }
    }

    fn word(&self, k: usize) -> Result<u16, String> {
        self.script
            .get(k)
            .copied()
            .ok_or_else(|| format!("prop node={} ran off its script at word={k}", self.node))
    }

    /// A literal or a random number below the next word.
    fn value(&mut self, s: &mut Skater) -> Result<Option<i32>, String> {
        let w = self.word(self.pc)?;
        self.pc += 1;
        Ok(match w {
            RANDOM => {
                let n = self.word(self.pc)?;
                self.pc += 1;
                Some(i32::from(s.rng.next(i32::from(n)) as i16))
            }
            w if w & VARIABLE != 0 => None,
            w => Some(i32::from(w)),
        })
    }

    fn run(&mut self, s: &mut Skater) -> Result<(), String> {
        loop {
            let op = self.word(self.pc)?;
            match op {
                END => return Ok(()),
                SCAN_LABELS => self.pc += 1,
                LABEL | COLLISION_TYPE => self.pc += 2,
                SET_MODEL => {
                    // the checksum sits at the next four byte boundary of the file
                    let at = ((self.script_offset + 2 * self.pc + 2 + 3) & !3) - self.script_offset;
                    let k = at / 2;
                    self.model =
                        Some(u32::from(self.word(k)?) | u32::from(self.word(k + 1)?) << 16);
                    self.hidden = false;
                    self.pc = k + 2;
                }
                SET_ANGLES
                    if (1..4).all(|n| {
                        self.script
                            .get(self.pc + n)
                            .is_some_and(|w| w & VARIABLE == 0)
                    }) =>
                {
                    let k = self.pc;
                    self.angles =
                        [self.word(k + 1)?, self.word(k + 2)?, self.word(k + 3)?].map(|w| w as i16);
                    self.pc += 4;
                }
                WAIT => {
                    let at = self.pc;
                    self.pc += 1;
                    match self.value(s)? {
                        Some(v) => {
                            self.wait = (v << 12) as u32;
                            return Ok(());
                        }
                        None => {
                            self.pc = at;
                            self.frozen_at = Some(op);
                            return Ok(());
                        }
                    }
                }
                op => {
                    self.frozen_at = Some(op);
                    return Ok(());
                }
            }
        }
    }

    /// Waits count down in frame time, then the script goes on.
    pub(super) fn update(&mut self, s: &mut Skater, hold: i32) -> Result<(), String> {
        if !self.started {
            self.started = true;
            return self.run(s);
        }
        if self.frozen_at.is_some() || self.wait == 0 {
            return Ok(());
        }
        self.wait = self.wait.saturating_sub(hold as u32);
        if self.wait == 0 {
            self.run(s)?;
        }
        Ok(())
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
