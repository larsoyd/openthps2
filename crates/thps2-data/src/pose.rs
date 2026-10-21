use crate::{anim::ANIMATION_CHUNK, psx::Scene};

const HIERARCHY_CHUNK: u32 = 0x5245_4948;

/// One part's place in a pose: its absolute rotation and its offset in the object's frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PoseRecord {
    pub m: [i16; 9],
    pub t: [i16; 3],
}

/// The animation stream and part tree of a bank: each anim holds, per part, three angles and an
/// offset from the parent, one value per frame.
pub struct Rig {
    stream: Vec<u8>,
    parents: Vec<u16>,
    order: Vec<usize>,
}

fn word(data: &[u8], at: usize) -> Result<u32, String> {
    data.get(at..at + 4)
        .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
        .ok_or_else(|| format!("[pose] word at={at} past len={}", data.len()))
}

/// Root first, then each level of children in part order.
fn calc_order(parents: &[u16]) -> Result<Vec<usize>, String> {
    let roots: Vec<usize> = (0..parents.len())
        .filter(|&p| usize::from(parents[p]) == p)
        .collect();
    let [root] = roots[..] else {
        return Err(format!("[pose] roots={roots:?} need exactly one"));
    };
    let mut order = vec![root];
    let mut k = 0;
    while order.len() < parents.len() {
        let parent = *order
            .get(k)
            .ok_or_else(|| format!("[pose] parts={} unreachable from root", parents.len()))?;
        order
            .extend((0..parents.len()).filter(|&c| c != root && usize::from(parents[c]) == parent));
        k += 1;
    }
    Ok(order)
}

/// Bit packed key deltas, read most significant bit first and sign extended.
struct Deltas {
    width: u32,
    bit: u32,
}

impl Deltas {
    fn next(&mut self, src: &[u8], at: &mut usize) -> i32 {
        let byte = |i: usize| u32::from(src.get(i).copied().unwrap_or(0));
        let v = (byte(*at) << 16) | (byte(*at + 1) << 8) | byte(*at + 2);
        let v = ((v << (self.bit + 8)) as i32) >> (32 - self.width);
        self.bit += self.width;
        *at += (self.bit >> 3) as usize;
        self.bit &= 7;
        v
    }
}

/// Reads one track, a value per frame, keys every step frames with straight lines between.
fn track(src: &[u8], at: &mut usize, out: &mut [i16]) -> Result<(), String> {
    let half = |i: usize| -> Result<i16, String> {
        src.get(i..i + 2)
            .map(|b| i16::from_le_bytes([b[0], b[1]]))
            .ok_or_else(|| format!("[pose] track value at={i} past len={}", src.len()))
    };
    let count = out.len();
    let head = *src
        .get(*at)
        .ok_or_else(|| format!("[pose] track header at={} past len={}", *at, src.len()))?;
    *at += 1;
    let step = usize::from(head >> 4) + 1;
    let kind = head & 0xf;
    let (keys, rest) = if step >= 2 {
        let q = (count - 1) / step;
        (q, count - q * step - 1)
    } else {
        (count - 1, 0)
    };
    match kind {
        15 => out.fill(0),
        14 => {
            out.fill(half(*at)?);
            *at += 2;
        }
        _ => {
            let mut deltas = Deltas {
                width: u32::from(kind) + 1,
                bit: 0,
            };
            let mut prev = half(*at)?;
            *at += 2;
            out[0] = prev;
            let mut n = 1;
            for span in std::iter::repeat_n(step, keys).chain((rest > 0).then_some(rest)) {
                let dv = if kind == 0 {
                    let key = half(*at)?;
                    *at += 2;
                    i32::from(key) - i32::from(prev)
                } else {
                    deltas.next(src, at)
                };
                let d = dv / span as i32;
                let mut c = i32::from(prev);
                for _ in 1..span {
                    c += d;
                    out[n] = c as i16;
                    n += 1;
                }
                prev = (i32::from(prev) + dv) as i16;
                out[n] = prev;
                n += 1;
            }
            if kind != 0 && deltas.bit != 0 {
                *at += 1;
            }
        }
    }
    Ok(())
}

/// The fixed point sine and cosine of a 4096 step angle, truncated like the original's.
fn trig(angle: i16) -> (i32, i32) {
    let x = f64::from(angle) * f64::from(1.0f32 / 4096.0) * f64::from(std::f32::consts::TAU);
    ((x.sin() * 4096.0) as i32, (x.cos() * 4096.0) as i32)
}

fn sat16(v: i32) -> i16 {
    v.clamp(-32768, 32767) as i16
}

/// m times r, each column through the transform unit with saturation.
fn mul(m: &[i16; 9], r: &[i32; 9]) -> [i16; 9] {
    std::array::from_fn(|k| {
        let (i, j) = (k / 3, k % 3);
        let sum: i32 = (0..3).map(|n| i32::from(m[3 * i + n]) * r[3 * n + j]).sum();
        sat16(sum >> 12)
    })
}

/// Rotation about y, then x, then z, each applied on the right.
pub fn euler([ax, ay, az]: [i16; 3]) -> [i16; 9] {
    let one = 4096;
    let mut m = [0i16; 9];
    (m[0], m[4], m[8]) = (one as i16, one as i16, one as i16);
    let (s, c) = trig(ay);
    m = mul(&m, &[c, 0, s, 0, one, 0, -s, 0, c]);
    let (s, c) = trig(ax);
    m = mul(&m, &[one, 0, 0, 0, c, -s, 0, s, c]);
    let (s, c) = trig(az);
    mul(&m, &[c, -s, 0, s, c, 0, 0, 0, one])
}

impl Rig {
    pub fn from_scene(scene: &Scene) -> Result<Self, String> {
        let chunk = |kind: u32| {
            scene
                .chunks
                .iter()
                .find(|c| c.kind == kind)
                .map(|c| &c.payload)
                .ok_or_else(|| format!("[pose] scene has no chunk kind={kind:#x}"))
        };
        let stream = chunk(ANIMATION_CHUNK)?.clone();
        let hierarchy = chunk(HIERARCHY_CHUNK)?;
        let parts = scene.models.len();
        if hierarchy.len() < 2 * parts {
            return Err(format!(
                "[pose] hierarchy bytes={} parts={parts}",
                hierarchy.len()
            ));
        }
        let parents: Vec<u16> = hierarchy[..2 * parts]
            .chunks(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect();
        if let Some(p) = parents.iter().position(|&p| usize::from(p) >= parts) {
            return Err(format!(
                "[pose] part={p} parent={} past parts={parts}",
                parents[p]
            ));
        }
        Ok(Self {
            order: calc_order(&parents)?,
            stream,
            parents,
        })
    }

    pub fn anims(&self) -> Result<usize, String> {
        Ok(word(&self.stream, 0)? as usize)
    }

    /// Every frame of an anim: per part three angles then the offset.
    pub fn decode(&self, anim: usize) -> Result<Vec<Vec<[i16; 6]>>, String> {
        let count = self.anims()?;
        if anim >= count {
            return Err(format!("[pose] anim={anim} count={count}"));
        }
        let mut at = word(&self.stream, 4 + 8 * anim)? as usize;
        let frames = word(&self.stream, 8 + 8 * anim)? as usize;
        if frames == 0 {
            return Err(format!("[pose] anim={anim} has no frames"));
        }
        let parts = self.parents.len();
        let mut clip = vec![vec![[0i16; 6]; parts]; frames];
        let mut values = vec![0i16; frames];
        for p in 0..parts {
            for k in 0..6 {
                track(&self.stream, &mut at, &mut values)
                    .map_err(|e| format!("{e} anim={anim} part={p} track={k}"))?;
                for (frame, v) in clip.iter_mut().zip(&values) {
                    frame[p][k] = *v;
                }
            }
        }
        Ok(clip)
    }

    /// Absolute rotations from each part's own angles, offsets turned by the parent's rotation.
    pub fn pose(&self, frame: &[[i16; 6]]) -> Vec<PoseRecord> {
        let mut pose = vec![PoseRecord::default(); self.parents.len()];
        for &c in &self.order {
            let v = frame[c];
            let m = euler([v[0], v[1], v[2]]);
            let parent = usize::from(self.parents[c]);
            let t = if parent == c {
                [v[3], v[4], v[5]]
            } else {
                let p = pose[parent];
                std::array::from_fn(|i| {
                    let sum: i32 = (0..3)
                        .map(|n| i32::from(p.m[3 * i + n]) * i32::from(v[3 + n]))
                        .sum();
                    sat16(sum >> 12).wrapping_add(p.t[i])
                })
            };
            pose[c] = PoseRecord { m, t };
        }
        pose
    }
}
