/// One pre-transformed vertex as the PC renderer submits it: screen position, depth, reciprocal w,
/// ARGB diffuse and texture coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenVertex {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub rhw: f32,
    pub diffuse: u32,
    pub uv: [f32; 2],
}

/// A 16-bit ARGB1555 texture surface, rows of width texels.
#[derive(Clone, Debug)]
pub struct Texture {
    pub width: usize,
    pub height: usize,
    pub texels: Vec<u16>,
}

/// The fixed function state the game sets, with Direct3D 7 numbering for blend factors, the depth
/// test and the texture stage operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrawState {
    pub blend: bool,
    pub src: u32,
    pub dst: u32,
    pub depth_test: bool,
    pub depth_write: bool,
    pub depth_func: u32,
    pub colour_op: u32,
    pub alpha_op: u32,
    pub wrap: bool,
    pub gouraud: bool,
    /// Device viewport x, y, width, height, the whole target when none.
    pub viewport: Option<[usize; 4]>,
}

impl Default for DrawState {
    fn default() -> Self {
        Self {
            blend: false,
            src: BLEND_ONE,
            dst: BLEND_ZERO,
            depth_test: false,
            depth_write: false,
            depth_func: 4,
            colour_op: OP_MODULATE,
            alpha_op: OP_SELECT_ARG1,
            wrap: true,
            gouraud: true,
            viewport: None,
        }
    }
}

pub const BLEND_ZERO: u32 = 1;
pub const BLEND_ONE: u32 = 2;
const BLEND_SRC_COLOUR: u32 = 3;
const BLEND_INV_SRC_COLOUR: u32 = 4;
const BLEND_SRC_ALPHA: u32 = 5;
const BLEND_INV_SRC_ALPHA: u32 = 6;
const OP_SELECT_ARG1: u32 = 2;
const OP_SELECT_ARG2: u32 = 3;
const OP_MODULATE: u32 = 4;

/// An RGB565 colour buffer with a 16-bit depth buffer, like the game's 640x480x16 mode.
pub struct Raster {
    pub width: usize,
    pub height: usize,
    pub colour: Vec<u16>,
    pub depth: Vec<u16>,
}

type Rgba = [f32; 4];

fn unpack_argb(c: u32) -> Rgba {
    [16, 8, 0, 24].map(|s| ((c >> s) & 0xff) as f32 / 255.0)
}

/// A channel widened to 8 bits by repeating its high bits, as a fraction.
fn unorm(v: u32, bits: u32) -> f32 {
    ((v << (8 - bits)) | (v >> (2 * bits - 8))) as f32 / 255.0
}

fn texel(t: &Texture, x: usize, y: usize) -> Rgba {
    let p = u32::from(t.texels[y * t.width + x]);
    [
        unorm((p >> 10) & 0x1f, 5),
        unorm((p >> 5) & 0x1f, 5),
        unorm(p & 0x1f, 5),
        if p & 0x8000 != 0 { 1.0 } else { 0.0 },
    ]
}

/// Bilinear weights keep eight fraction bits.
fn sample(t: &Texture, [u, v]: [f32; 2], wrap: bool) -> Rgba {
    let axis = |c: f32, size: usize| -> (usize, usize, f32) {
        let x = (c * size as f32 * 256.0 - 128.0).floor() as i64;
        let at = |k: i64| -> usize {
            if wrap {
                k.rem_euclid(size as i64) as usize
            } else {
                k.clamp(0, size as i64 - 1) as usize
            }
        };
        (at(x >> 8), at((x >> 8) + 1), (x & 0xff) as f32 / 256.0)
    };
    let (x0, x1, fx) = axis(u, t.width);
    let (y0, y1, fy) = axis(v, t.height);
    let (a, b, c, d) = (
        texel(t, x0, y0),
        texel(t, x1, y0),
        texel(t, x0, y1),
        texel(t, x1, y1),
    );
    let b8 = |x: f32| (x * 255.0).round() as i32;
    let lerp = |p: i32, q: i32, w: f32| p + (((q - p) * (w * 256.0) as i32 + 128) >> 8);
    std::array::from_fn(|i| {
        let top = lerp(b8(a[i]), b8(b[i]), fx);
        let bottom = lerp(b8(c[i]), b8(d[i]), fx);
        lerp(top, bottom, fy) as f32 / 255.0
    })
}

fn factor(kind: u32, src: Rgba) -> Result<Rgba, String> {
    Ok(match kind {
        BLEND_ZERO => [0.0; 4],
        BLEND_ONE => [1.0; 4],
        BLEND_SRC_COLOUR => src,
        BLEND_INV_SRC_COLOUR => src.map(|c| 1.0 - c),
        BLEND_SRC_ALPHA => [src[3]; 4],
        BLEND_INV_SRC_ALPHA => [1.0 - src[3]; 4],
        k => return Err(format!("[raster] blend factor={k} unsupported")),
    })
}

impl Raster {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            colour: vec![0; width * height],
            depth: vec![0; width * height],
        }
    }

    pub fn clear(&mut self, argb: u32, depth: f32) {
        self.colour.fill(pack(unpack_argb(argb)));
        self.depth.fill(quantise_depth(depth));
    }

    pub fn fan(
        &mut self,
        v: &[ScreenVertex],
        state: &DrawState,
        texture: Option<&Texture>,
    ) -> Result<(), String> {
        for i in 1..v.len().saturating_sub(1) {
            self.triangle([v[0], v[i], v[i + 1]], state, texture)?;
        }
        Ok(())
    }

    fn triangle(
        &mut self,
        v: [ScreenVertex; 3],
        state: &DrawState,
        texture: Option<&Texture>,
    ) -> Result<(), String> {
        let edge = |a: &ScreenVertex, b: &ScreenVertex, x: f32, y: f32| {
            (b.x - a.x) * (y - a.y) - (b.y - a.y) * (x - a.x)
        };
        let area = edge(&v[0], &v[1], v[2].x, v[2].y);
        if area == 0.0 {
            return Ok(());
        }
        let min_x = v
            .iter()
            .map(|p| p.x)
            .fold(f32::MAX, f32::min)
            .floor()
            .max(0.0) as usize;
        let max_x =
            (v.iter().map(|p| p.x).fold(f32::MIN, f32::max).ceil() as usize).min(self.width);
        let min_y = v
            .iter()
            .map(|p| p.y)
            .fold(f32::MAX, f32::min)
            .floor()
            .max(0.0) as usize;
        let max_y =
            (v.iter().map(|p| p.y).fold(f32::MIN, f32::max).ceil() as usize).min(self.height);
        let [vx, vy, vw, vh] = state.viewport.unwrap_or([0, 0, self.width, self.height]);
        let (min_x, max_x) = (min_x.max(vx), max_x.min(vx + vw));
        let (min_y, max_y) = (min_y.max(vy), max_y.min(vy + vh));
        let colours = v.map(|p| unpack_argb(p.diffuse));
        for py in min_y..max_y {
            for px in min_x..max_x {
                // pixel centres sit on integer coordinates
                let (x, y) = (px as f32, py as f32);
                let w = [
                    edge(&v[1], &v[2], x, y) / area,
                    edge(&v[2], &v[0], x, y) / area,
                    edge(&v[0], &v[1], x, y) / area,
                ];
                if !inside(&v, &w, area) {
                    continue;
                }
                let q: f32 = (0..3).map(|i| w[i] * v[i].rhw).sum();
                let persp = |f: &dyn Fn(usize) -> f32| {
                    (0..3).map(|i| w[i] * v[i].rhw * f(i)).sum::<f32>() / q
                };
                let z: f32 = (0..3).map(|i| w[i] * v[i].z).sum();
                let at = py * self.width + px;
                let depth = quantise_depth(z);
                if state.depth_test && !depth_passes(state.depth_func, depth, self.depth[at])? {
                    continue;
                }
                let diffuse: Rgba = if state.gouraud {
                    std::array::from_fn(|c| persp(&|i| colours[i][c]))
                } else {
                    colours[0]
                };
                let tex = match texture {
                    Some(t) => sample(
                        t,
                        [persp(&|i| v[i].uv[0]), persp(&|i| v[i].uv[1])],
                        state.wrap,
                    ),
                    None => [1.0; 4],
                };
                let combine = |op: u32, c: usize| -> Result<f32, String> {
                    Ok(match op {
                        OP_SELECT_ARG1 => tex[c],
                        OP_SELECT_ARG2 => diffuse[c],
                        OP_MODULATE => tex[c] * diffuse[c],
                        o => return Err(format!("[raster] stage op={o} unsupported")),
                    })
                };
                let src = [
                    combine(state.colour_op, 0)?,
                    combine(state.colour_op, 1)?,
                    combine(state.colour_op, 2)?,
                    combine(state.alpha_op, 3)?,
                ];
                let out = if state.blend {
                    let dst = unpack565(self.colour[at]);
                    let (fs, fd) = (factor(state.src, src)?, factor(state.dst, src)?);
                    std::array::from_fn(|c| (src[c] * fs[c] + dst[c] * fd[c]).clamp(0.0, 1.0))
                } else {
                    src
                };
                self.colour[at] = pack(out);
                if state.depth_write {
                    self.depth[at] = depth;
                }
            }
        }
        Ok(())
    }
}

/// Top-left rule: a sample exactly on an edge counts only for top and left edges.
fn inside(v: &[ScreenVertex; 3], w: &[f32; 3], area: f32) -> bool {
    (0..3).all(|k| {
        if w[k] > 0.0 {
            return true;
        }
        if w[k] < 0.0 {
            return false;
        }
        let (a, b) = (&v[(k + 1) % 3], &v[(k + 2) % 3]);
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let dy = if area < 0.0 { -dy } else { dy };
        let dx = if area < 0.0 { -dx } else { dx };
        dy < 0.0 || (dy == 0.0 && dx > 0.0)
    })
}

fn depth_passes(func: u32, new: u16, old: u16) -> Result<bool, String> {
    Ok(match func {
        1 => false,
        2 => new < old,
        3 => new == old,
        4 => new <= old,
        5 => new > old,
        6 => new != old,
        7 => new >= old,
        8 => true,
        f => return Err(format!("[raster] depth func={f} unsupported")),
    })
}

fn quantise_depth(z: f32) -> u16 {
    (z.clamp(0.0, 1.0) * 65535.0).round() as u16
}

fn unpack565(p: u16) -> Rgba {
    let p = u32::from(p);
    [
        unorm(p >> 11, 5),
        unorm((p >> 5) & 0x3f, 6),
        unorm(p & 0x1f, 5),
        1.0,
    ]
}

/// Colours leave the pipeline as 8 bits a channel and the 16-bit buffer keeps the high bits.
fn pack(c: Rgba) -> u16 {
    let q = |v: f32, drop: u32| ((v.clamp(0.0, 1.0) * 255.0).round() as u16) >> drop;
    q(c[0], 3) << 11 | q(c[1], 2) << 5 | q(c[2], 3)
}
