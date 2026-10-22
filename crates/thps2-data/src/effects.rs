use crate::{
    camera::{rcos, rsin},
    collision::{CollisionWorld, Particle},
    pose::PoseRecord,
    raster::{BLEND_ONE, DrawState, ScreenVertex},
    render::View,
};

type V3 = [i32; 3];

/// The pose part blood sprays from, the head.
const HEAD: usize = 15;
const BLOOD_RGB: [u8; 3] = [0x48, 0, 0];
/// Spray aim: straight up, then spread.
const SPRAY_PITCH: i32 = 0xfc00 - 0x10000;
const SPRAY_FLOOR_ABOVE: i32 = 0x28000;
const SPRAY_FLOOR_BELOW: i32 = 0x320000;
const SPRAY_FLOOR_LIFT: i32 = 0x4000;
const SPRAY_LIFT: i32 = 0x1e000;

/// Where the skater's head is: the part's offset turned by the drawing matrix, its whole
/// units added back onto the skater's Q12 position.
pub fn head_hook(pose: &[PoseRecord], orientation: [i16; 9], position: V3) -> Option<V3> {
    let t = pose.get(HEAD)?.t.map(i32::from);
    Some(std::array::from_fn(|i| {
        let turned = (0..3)
            .map(|n| i32::from(orientation[3 * i + n]) * t[n])
            .sum::<i32>()
            >> 12;
        let whole = turned + (position[i] >> 12);
        ((whole << 12).wrapping_sub(position[i]) >> 4).wrapping_add(position[i])
    }))
}

/// Where a spray starts and the height its drops stop at: thirty units above the hook, and
/// four above the ground found below it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SprayOrigin {
    at: V3,
    floor: i32,
}

pub fn spray_origin(world: &CollisionWorld, hook: V3) -> Result<SprayOrigin, String> {
    let start = [hook[0], hook[1] - SPRAY_FLOOR_ABOVE, hook[2]];
    let end = [hook[0], hook[1] + SPRAY_FLOOR_BELOW, hook[2]];
    let ground = world
        .query(start, end)?
        .map_or(hook[1], |h| h.position_fixed[1]);
    Ok(SprayOrigin {
        at: [hook[0], hook[1] - SPRAY_LIFT, hook[2]],
        floor: ground - SPRAY_FLOOR_LIFT,
    })
}

/// The flight of a spark the run threw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flight {
    Blood(Blood),
    Rail(RailSpark),
}

/// Two argument arc tangent in 4096 units per turn, truncated.
fn ratan2(y: i32, x: i32) -> i32 {
    (f64::from(y).atan2(f64::from(x)) * 651.898_628_763_671_2) as i32
}

/// Pitch and yaw of a drawing matrix, as the skater's angles are kept.
pub fn aim(m: [i16; 9]) -> [i32; 2] {
    let m = m.map(i32::from);
    let flat = crate::skater::sqrt_int(m[8] * m[8] + m[2] * m[2]);
    [ratan2(-m[5], flat), ratan2(m[2], m[8])]
}

const RAIL_SPARK_DROP: i32 = 0x19000;
const RAIL_SPARK_FALL: i32 = 0x1000;
const RAIL_SPARK_RED: u8 = 0x80;

/// A rail spark: a point below the skater's feet that it carries along, thrown along the board's
/// aim, drawn as a line from where it was to where it is, fading from red to black.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RailSpark {
    offset: V3,
    velocity: V3,
    pub ends: [V3; 2],
    pub green: u8,
}

impl RailSpark {
    pub fn thrown(at: V3, aim: [i32; 2], spread: [i32; 2], speed: i32, green: i32) -> Self {
        let (pitch, yaw) = (aim[0] + spread[0], aim[1] + spread[1]);
        let reach = (rcos(pitch) * speed) >> 12;
        let offset = [0, RAIL_SPARK_DROP, 0];
        let start = std::array::from_fn(|a| offset[a] + at[a]);
        Self {
            offset,
            velocity: [
                -(reach * rsin(yaw)),
                rsin(pitch) * speed,
                -(reach * rcos(yaw)),
            ],
            ends: [start, start],
            green: green as u8,
        }
    }

    /// Moves with the skater: both ends sit on its new position, the old offset and the new.
    pub fn update(&mut self, skater: V3) {
        let place = |o: V3| -> V3 { std::array::from_fn(|a| o[a].wrapping_add(skater[a])) };
        self.ends[1] = place(self.offset);
        let accel = [0, RAIL_SPARK_FALL, 0];
        let axes = self.offset.iter_mut().zip(&mut self.velocity).zip(accel);
        for ((o, v), a) in axes {
            *o = o.wrapping_add(*v);
            let sped = v.wrapping_add(a);
            *v = sped - (sped >> 3);
        }
        self.ends[0] = place(self.offset);
    }
}

/// A drop of a blood spray: Q12 flight with its own fall, damped by an eighth a frame, a dot
/// of one or two pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Blood {
    pub position: V3,
    velocity: V3,
    gravity: i32,
    floor: i32,
    pub size: i16,
}

impl Blood {
    /// The i-th drop: its speed and fall grow with i, its aim spread around straight up, and it
    /// leaves with the skater's velocity.
    pub fn thrown(origin: SprayOrigin, i: usize, spread: [i32; 2], carried: V3) -> Self {
        let speed = (i as i32 >> 2) + 0xe;
        let (pitch, yaw) = (SPRAY_PITCH + spread[0], spread[1]);
        let reach = (rcos(pitch) * speed) >> 12;
        let aim = [
            -(reach * rsin(yaw)),
            rsin(pitch) * speed,
            -(reach * rcos(yaw)),
        ];
        Self {
            position: origin.at,
            velocity: std::array::from_fn(|a| aim[a].wrapping_add(carried[a])),
            gravity: 0x2800 + 0x100 * i as i32,
            floor: origin.floor,
            size: (i & 1) as i16 + 1,
        }
    }

    /// A drop that reaches its floor stops there, otherwise it moves and its speed loses an eighth.
    pub fn update(&mut self) {
        let y = self.position[1].wrapping_add(self.velocity[1]);
        if self.floor != 0 && y > self.floor {
            self.position[1] = self.floor;
            return;
        }
        let accel = [0, self.gravity, 0];
        let axes = self.position.iter_mut().zip(&mut self.velocity).zip(accel);
        for ((p, v), a) in axes {
            *p = p.wrapping_add(*v);
            let sped = v.wrapping_add(a);
            *v = sped - (sped >> 3);
        }
    }
}

const SLOT_MASK: i32 = 0xfff;
/// The effects pass sets draw mode abr 1 before its primitives, so semi ones add.
const EFFECTS_ABR: u8 = 1;
/// The handlers drop a primitive wider or taller than these.
const MAX_SPAN: [f32; 2] = [1024.0, 512.0];

/// A PS1 primitive the effects pass puts in the ordering table: its screen corners, flat colour
/// or a colour per corner for a line, slot and the depth the PC handler draws it at.
#[derive(Clone, Debug, PartialEq)]
pub struct Prim {
    pub corners: Vec<[i16; 2]>,
    pub colours: Vec<[u8; 3]>,
    pub semi: bool,
    pub abr: u8,
    pub slot: i32,
    pub z: f32,
}

impl View {
    /// The GTE perspective transform of a whole unit offset from the eye: the screen point,
    /// clamped to low..0x3ff on each axis, and the unclamped view depth.
    pub(crate) fn gte(&self, p: [i16; 3], low: i16) -> ([i16; 2], i32) {
        let r = &self.rows;
        let mac: [i32; 3] = std::array::from_fn(|i| {
            (0..3).fold(0i32, |acc, j| {
                acc + i32::from(r[3 * i + j]) * i32::from(p[j])
            }) >> 12
        });
        let ir = mac.map(|m| m.clamp(-0x8000, 0x7fff));
        let sz = mac[2].clamp(0, 0xffff);
        if sz == 0 {
            return ([0; 2], mac[2]);
        }
        let q = (self.h << 12) / sz;
        let centre = [self.viewport.width / 2, self.viewport.height / 2];
        let xy = std::array::from_fn(|a| {
            let s = (ir[a].wrapping_mul(q) >> 12) as i16;
            let s = s.wrapping_add(centre[a] as i16);
            if s < low { low } else { s.min(0x3ff) }
        });
        (xy, mac[2])
    }

    /// The table slot of a view depth and the depth the handler draws at, near over four slots.
    fn slot_depth(&self, depth: i32) -> (i32, f32) {
        let slot = (depth >> 2) & SLOT_MASK;
        let scaled = (slot * 4) as f32;
        let near = self.viewport.near as f32;
        (slot, if scaled <= 0.0 { near } else { near / scaled })
    }

    /// A whole unit offset from the eye, cut to sixteen bits like the pass's shorts.
    fn relative(&self, q12: [i32; 3]) -> [i16; 3] {
        std::array::from_fn(|a| ((q12[a] >> 12) as i16).wrapping_sub(self.eye[a] as i16))
    }

    /// A falling glass triangle, flat and additive, slotted by its third corner.
    pub fn particle(&self, p: &Particle) -> Prim {
        let mut depth = 0;
        let corners = p
            .corners
            .iter()
            .map(|&c| {
                let (xy, d) = self.gte(self.relative(c), -0x3ff);
                depth = d;
                xy
            })
            .collect();
        let (slot, z) = self.slot_depth(depth);
        Prim {
            corners,
            colours: vec![p.shown; 3],
            semi: true,
            abr: EFFECTS_ABR,
            slot,
            z,
        }
    }
}

impl View {
    /// A blood drop as a tile of its size, opaque, between the near and far planes.
    pub fn blood(&self, b: &Blood) -> Option<Prim> {
        let ([x, y], depth) = self.gte(self.relative(b.position), -0x400);
        if depth < self.viewport.near || depth > self.viewport.far {
            return None;
        }
        let (slot, z) = self.slot_depth(depth);
        let (right, down) = (x.wrapping_add(b.size), y.wrapping_add(b.size));
        Some(Prim {
            corners: vec![[x, y], [right, y], [right, down], [x, down]],
            colours: vec![BLOOD_RGB; 4],
            semi: false,
            abr: EFFECTS_ABR,
            slot,
            z,
        })
    }
}

impl View {
    /// A rail spark's line when both ends sit between the near and far planes, additive, slotted
    /// by the nearer end.
    pub fn rail_spark(&self, s: &RailSpark) -> Option<Prim> {
        let [(now, d0), (was, d1)] = s.ends.map(|e| self.gte(self.relative(e), -0x400));
        let (near, far) = (self.viewport.near, self.viewport.far);
        if [d0, d1].iter().any(|&d| d < near || d > far) {
            return None;
        }
        let (slot, z) = self.slot_depth(d0.min(d1));
        Some(Prim {
            corners: vec![now, was],
            colours: vec![[RAIL_SPARK_RED, s.green, 0], [0; 3]],
            semi: true,
            abr: EFFECTS_ABR,
            slot,
            z,
        })
    }
}

impl Prim {
    /// The draw the ordering table walker issues: gamma corrected flat colour, alpha and blend
    /// from the semi bit and abr, depth tested without writes when semi.
    pub fn draw(&self, gamma: &[u8; 256]) -> Option<(Vec<ScreenVertex>, DrawState)> {
        let span = |a: usize| {
            let v = self.corners.iter().map(|c| f32::from(c[a]));
            v.clone().fold(f32::MIN, f32::max) - v.fold(f32::MAX, f32::min)
        };
        // the line handler draws without the size gate
        if self.corners.len() != 2 && (span(0) > MAX_SPAN[0] || span(1) > MAX_SPAN[1]) {
            return None;
        }
        let (alpha, blend) = match (self.semi, self.abr) {
            (false, _) => (0xff, 1),
            (true, 0) => (0x80, 1),
            (true, 1) => (0xff, 2),
            (true, 2) => (0, 4),
            (true, _) => (0x40, 2),
        };
        let vertices = self
            .corners
            .iter()
            .zip(&self.colours)
            .map(|(c, rgb)| {
                let [r, g, b] = rgb.map(|c| u32::from(gamma[usize::from(c)]));
                (c, alpha << 24 | r << 16 | g << 8 | b)
            })
            .map(|(c, diffuse)| ScreenVertex {
                x: f32::from(c[0]),
                y: f32::from(c[1]),
                z: self.z,
                rhw: 1.0,
                diffuse,
                uv: [0.0; 2],
            })
            .collect();
        let (src, dst) = match blend {
            1 => (5, 6),
            2 => (5, BLEND_ONE),
            _ => (1, 4),
        };
        let state = DrawState {
            blend: true,
            src,
            dst,
            depth_test: true,
            depth_write: !self.semi,
            depth_func: 7,
            colour_op: 3,
            alpha_op: 3,
            wrap: false,
            gouraud: true,
            viewport: None,
        };
        Some((vertices, state))
    }
}
