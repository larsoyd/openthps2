use crate::{
    collision::Particle,
    raster::{BLEND_ONE, DrawState, ScreenVertex},
    render::View,
};

const SLOT_MASK: i32 = 0xfff;
/// The effects pass sets draw mode abr 1 before its primitives, so semi ones add.
const EFFECTS_ABR: u8 = 1;
/// The handlers drop a primitive wider or taller than these.
const MAX_SPAN: [f32; 2] = [1024.0, 512.0];

/// A PS1 primitive the effects pass puts in the ordering table: its screen corners, flat colour,
/// slot and the depth the PC handler draws it at.
#[derive(Clone, Debug, PartialEq)]
pub struct Prim {
    pub corners: Vec<[i16; 2]>,
    pub rgb: [u8; 3],
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
            rgb: p.shown,
            semi: true,
            abr: EFFECTS_ABR,
            slot,
            z,
        }
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
        if span(0) > MAX_SPAN[0] || span(1) > MAX_SPAN[1] {
            return None;
        }
        let (alpha, blend) = match (self.semi, self.abr) {
            (false, _) => (0xff, 1),
            (true, 0) => (0x80, 1),
            (true, 1) => (0xff, 2),
            (true, 2) => (0, 4),
            (true, _) => (0x40, 2),
        };
        let [r, g, b] = self.rgb.map(|c| u32::from(gamma[usize::from(c)]));
        let diffuse = alpha << 24 | r << 16 | g << 8 | b;
        let vertices = self
            .corners
            .iter()
            .map(|c| ScreenVertex {
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
