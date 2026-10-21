use super::*;

pub(super) const SCRIPT_OBJECT: u16 = 0xcb;
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
