use crate::{
    camera::Matrix,
    psx::{Face, Item, Model, Scene},
    raster::ScreenVertex,
};

/// Every model transform runs with the x87 rounding to single precision, so f32 ops in the
/// original's order give its results.
const Q12: f32 = 0.000_244_140_62;
const SLOTS: i32 = 0xfff;
const Z_CAP: f32 = 0.99;
const COLOUR_TABLE_KIND: u32 = u32::from_le_bytes(*b"RGBs");
/// Quad corners in fan order.
const QUAD_FAN: [usize; 4] = [0, 1, 3, 2];
/// Item flag for geometry past the far plane, left out of the fog.
const OPEN_FAR: u16 = 0x2000;

/// The viewport and per level render settings the view is built from.
#[derive(Clone, Copy, Debug)]
pub struct Viewport {
    pub width: i32,
    pub height: i32,
    pub near: i32,
    /// Far plane of the culling frustum.
    pub far: i32,
    pub fov: i32,
    pub fog_start: i32,
    pub fog_scale: f32,
}

/// The camera as the renderer uses it: eye in whole units and the view rows.
#[derive(Clone, Copy, Debug)]
pub struct View {
    pub eye: [i32; 3],
    pub rows: Matrix,
    pub h: i32,
    pub viewport: Viewport,
}

impl View {
    /// View rows are the camera matrix transposed, the aspect scale is one at 4:3.
    pub fn new(eye: [i32; 3], camera: &Matrix, viewport: Viewport) -> Self {
        let m = camera;
        let rows = [m[0], m[3], m[6], m[1], m[4], m[7], m[2], m[5], m[8]];
        let h = ((viewport.width << 11) & !0xfff) / ((viewport.fov << 12) / 0x1000);
        Self {
            eye,
            rows,
            h,
            viewport,
        }
    }
}

/// A projected vertex: screen position, view depth, H over depth and its clip outcode.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Projected {
    pub sx: f32,
    pub sy: f32,
    pub z: f32,
    pub hz: f32,
    pub outcode: u32,
}

/// How the faces of one model are coloured and placed in the ordering table.
pub(crate) struct Faces<'a> {
    pub textures: &'a [TextureInfo],
    pub colours: &'a [u8],
    /// Per vertex light of lit models, none for unlit ones.
    pub lights: Option<&'a [u32]>,
    pub bias: f32,
    /// Skater parts take seven tenths of their depth for the table and the depth buffer.
    pub figure: bool,
    /// A mirrored model keeps the faces the others drop.
    pub mirrored: bool,
}

/// One polygon record as the 3D pass queues it, in its ordering table slot.
#[derive(Clone, Debug)]
pub struct Poly {
    pub item: usize,
    pub face: usize,
    pub flags: u32,
    pub fogged: bool,
    pub texture: Option<u32>,
    pub vertices: Vec<ScreenVertex>,
    pub slot: i32,
}

/// What a level texture tells the renderer: its logical size and whether it has clear texels.
#[derive(Clone, Copy, Debug)]
pub struct TextureInfo {
    pub width: u16,
    pub height: u16,
    pub opaque: bool,
}

pub struct Level<'a> {
    pub scene: &'a Scene,
    pub textures: &'a [TextureInfo],
    /// Depth bias per model index.
    pub bias: &'a dyn Fn(usize) -> i32,
    pub hidden: &'a dyn Fn(usize) -> bool,
}

fn colour_table(scene: &Scene) -> Result<&[u8], String> {
    scene
        .chunks
        .iter()
        .find(|c| c.kind == COLOUR_TABLE_KIND)
        .map(|c| c.payload.as_slice())
        .filter(|p| p.len() == 1024)
        .ok_or_else(|| "[render] scene has no 256 entry colour table".into())
}

impl View {
    fn outcode(&self, sx: f32, sy: f32, z: f32) -> u32 {
        let vp = &self.viewport;
        let far = vp.far as f32 * vp.fog_scale;
        u32::from(sx < 0.0)
            | u32::from(sx >= vp.width as f32) << 1
            | u32::from(sy < 0.0) << 2
            | u32::from(sy >= vp.height as f32) << 3
            | u32::from(z < vp.near as f32) << 4
            | u32::from(z >= far) << 5
    }

    pub(crate) fn project(&self, r: &[[f32; 3]; 3], t: [f32; 3], v: [i16; 3]) -> Projected {
        let [vx, vy, vz] = v.map(f32::from);
        let row = |i: usize| ((r[i][2] * vz + r[i][1] * vy) + r[i][0] * vx) + t[i];
        let (x, y, z) = (row(0), row(1), row(2));
        let hz = self.h as f32 / z;
        let (cx, cy) = (
            self.viewport.width as f32 * 0.5,
            self.viewport.height as f32 * 0.5,
        );
        let (sx, sy) = (hz * x + cx, hz * y + cy);
        Projected {
            sx,
            sy,
            z,
            hz,
            outcode: self.outcode(sx, sy, z),
        }
    }

    /// A vertex set off in screen space from the line through two others, scaled by depth.
    fn offset(&self, a: Projected, b: Projected, off: f32) -> Projected {
        let dx = b.sx - a.sx;
        let dy = -(b.sy - a.sy);
        let len = (dy * dy + dx * dx).sqrt();
        let sx = ((dy / len) * off) * a.hz + a.sx;
        let sy = ((dx / len) * off) * a.hz + a.sy;
        Projected {
            sx,
            sy,
            z: a.z,
            hz: a.hz,
            outcode: self.outcode(sx, sy, a.z),
        }
    }

    /// Frustum planes in world space: far, near, then the four sides, normals in Q12 with the
    /// distance term the cull adds.
    fn planes(&self, camera: &Matrix) -> [([i32; 3], f64); 6] {
        let vp = &self.viewport;
        let ha = self.h * 0x1000;
        let hw = vp.width << 11;
        let l = crate::skater::sqrt_int((hw >> 12) * (hw >> 12) + (ha >> 12) * (ha >> 12));
        let (a, w) = (
            ((ha as u32) / l as u32) as i32,
            ((hw as u32) / l as u32) as i32,
        );
        let local = [
            [0, 0, -0x1000],
            [0, 0, 0x1000],
            [0, a, w],
            [0, -a, w],
            [a, 0, w],
            [-a, 0, w],
        ];
        let d = [
            f64::from(vp.far as f32 * vp.fog_scale),
            -f64::from(vp.near),
            0.0,
            0.0,
            0.0,
            0.0,
        ];
        std::array::from_fn(|p| {
            let n = local[p];
            let world = std::array::from_fn(|i| {
                let m = |j: usize| i32::from(camera[i * 3 + j]);
                ((m(0) * n[0] + m(1) * n[1] + m(2) * n[2]) >> 12).clamp(-0x8000, 0x7fff)
            });
            (world, d[p])
        })
    }

    /// Items whose bounding sphere, or failing that bounding box, lies outside a frustum plane.
    fn culled(
        &self,
        planes: &[([i32; 3], f64); 6],
        model: &Model,
        rel: [i32; 3],
        open_far: bool,
    ) -> bool {
        let r = rel.map(f64::from);
        if r[0].abs() >= 32767.0 || r[1].abs() >= 32767.0 || (!open_far && r[2].abs() >= 32767.0) {
            return true;
        }
        let planes = if open_far { &planes[1..] } else { &planes[..] };
        let distance = |p: &([i32; 3], f64), at: [f64; 3]| -> f64 {
            let n = p.0.map(|c| f64::from(c as f32 * Q12));
            n[0] * at[0] + n[1] * at[1] + n[2] * at[2] + p.1
        };
        let word = |at: usize| i32::from_le_bytes(model.header[at..at + 4].try_into().unwrap());
        let radius = f64::from(word(8) >> 12);
        if planes.iter().any(|p| distance(p, r) < -radius) {
            return true;
        }
        let faces = u16::from_le_bytes([model.header[6], model.header[7]]);
        if faces <= 10 {
            return false;
        }
        let pair = |at: usize| {
            [
                i16::from_le_bytes([model.header[at], model.header[at + 1]]),
                i16::from_le_bytes([model.header[at + 2], model.header[at + 3]]),
            ]
        };
        let (bx, by, bz) = (pair(0xc), pair(0x10), pair(0x14));
        let corners: Vec<[f64; 3]> = (0..8)
            .map(|k| {
                let pick = |b: [i16; 2], bit: usize| {
                    f64::from(if k >> bit & 1 == 0 {
                        b[0].min(b[1])
                    } else {
                        b[0].max(b[1])
                    })
                };
                [r[0] + pick(bx, 0), r[1] + pick(by, 1), r[2] + pick(bz, 2)]
            })
            .collect();
        planes
            .iter()
            .any(|p| corners.iter().all(|&c| distance(p, c) < -radius))
    }

    /// Polygons of the level items in submission order.
    pub fn level(&self, level: &Level, camera: &Matrix) -> Result<Vec<Poly>, String> {
        let colours = colour_table(level.scene)?;
        let planes = self.planes(camera);
        let rf: [[f32; 3]; 3] =
            std::array::from_fn(|i| std::array::from_fn(|j| f32::from(self.rows[i * 3 + j]) * Q12));
        let mut polys = Vec::new();
        for (index, item) in level.scene.items.iter().enumerate() {
            if (level.hidden)(index) {
                continue;
            }
            let rel: [i32; 3] =
                std::array::from_fn(|i| (item.position_fixed[i] >> 12) - self.eye[i]);
            let base = level
                .scene
                .models
                .get(item.model_index as usize)
                .ok_or_else(|| {
                    format!("[render] item={index} model={} missing", item.model_index)
                })?;
            let open_far = item.flags[0] & OPEN_FAR != 0;
            if self.culled(&planes, base, rel, open_far) {
                continue;
            }
            // these items ignore the far plane and the fog
            let item_view = if open_far {
                View {
                    viewport: Viewport {
                        far: 0x7fff,
                        ..self.viewport
                    },
                    ..*self
                }
            } else {
                *self
            };
            let Some(model) = self.lod(level.scene, item.model_index as usize, rel)? else {
                continue;
            };
            check_item(index, item, model)?;
            let [rx, ry, rz] = rel.map(|c| c as f32);
            let t: [f32; 3] =
                std::array::from_fn(|i| (rf[i][2] * rz + rf[i][1] * ry) + rf[i][0] * rx);
            let mut projected: Vec<Projected> = Vec::with_capacity(model.vertices.len());
            for (vi, v) in model.vertices.iter().enumerate() {
                let p = match v.flags {
                    0 => item_view.project(&rf, t, v.words),
                    0x10 => {
                        let at = |w: i16| projected.get((w as u16 >> 3) as usize).copied();
                        let (Some(a), Some(b)) = (at(v.words[0]), at(v.words[1])) else {
                            return Err(format!(
                                "[render] item={index} vertex={vi} offset anchors not projected"
                            ));
                        };
                        item_view.offset(a, b, f32::from(v.words[2]))
                    }
                    f => {
                        return Err(format!(
                            "[render] item={index} vertex={vi} flags={f:#x} unsupported"
                        ));
                    }
                };
                projected.push(p);
            }
            let faces = Faces {
                textures: level.textures,
                colours,
                lights: None,
                bias: (level.bias)(item.model_index as usize) as f32,
                figure: false,
                mirrored: false,
            };
            for (fi, face) in model.faces.iter().enumerate() {
                let at = polys.len();
                item_view
                    .face(&faces, &projected, face, &mut polys)
                    .map_err(|e| format!("[render] item={index} face={fi} {e}"))?;
                for p in &mut polys[at..] {
                    (p.item, p.face, p.fogged) = (index, fi, !open_far);
                }
            }
        }
        Ok(polys)
    }

    pub(crate) fn face(
        &self,
        ctx: &Faces,
        projected: &[Projected],
        face: &Face,
        polys: &mut Vec<Poly>,
    ) -> Result<(), String> {
        let (colours, bias) = (ctx.colours, ctx.bias);
        let source = face.flags & 0xffff;
        let f = if source & 0x40 != 0 {
            source
        } else {
            source ^ 0x80
        };
        if f & 0xc0 == 0 {
            return Ok(());
        }
        let count = if f & 0x10 != 0 { 3 } else { 4 };
        let raw = &face.raw;
        let light = |k: usize| -> Result<Option<u32>, String> {
            let at = match (f & 0xc, ctx.lights) {
                (0, _) => return Ok(None),
                (0xc, Some(_)) => usize::from(raw[4 + k]),
                (4, Some(_)) => usize::from(u16::from_le_bytes([raw[0xc], raw[0xd]]) >> 3),
                _ => return Err(format!("lit face flags={f:#x} unsupported")),
            };
            let lights = ctx.lights.unwrap_or_default();
            lights
                .get(at)
                .copied()
                .map(Some)
                .ok_or_else(|| format!("light={at} lights={}", lights.len()))
        };
        let colour = |k: usize| -> Result<u32, String> {
            let mut c = if f & 0x800 != 0 {
                let e = colours
                    .get(raw[8 + k] as usize * 4..)
                    .ok_or("gouraud face without a colour table")?;
                0xff00_0000 | u32::from(e[0]) << 16 | u32::from(e[1]) << 8 | u32::from(e[2])
            } else {
                0xff00_0000 | u32::from(raw[8]) << 16 | u32::from(raw[9]) << 8 | u32::from(raw[10])
            };
            if let Some(l) = light(k)? {
                let lit = |s: u32| ((c >> s & 0xff) * (l >> s & 0xff) / 255) << s;
                c = 0xff00_0000 | lit(16) | lit(8) | lit(0);
            }
            Ok(if f & 0x1c0 == 0x140 {
                let five = |s: u32| ((c >> s & 0xff) * 5).min(255) << s;
                0xff00_0000 | five(16) | five(8) | five(0)
            } else {
                c
            })
        };
        let texture = if f & 1 != 0 {
            let index = face.texture_index.ok_or("textured face without texture")?;
            Some((
                index,
                *ctx.textures
                    .get(index as usize)
                    .ok_or_else(|| format!("texture={index} missing"))?,
            ))
        } else {
            None
        };
        let order: &[usize] = if count == 3 { &[0, 1, 2] } else { &QUAD_FAN };
        let corners: Vec<(Projected, ScreenVertex)> = order
            .iter()
            .map(|&c| -> Result<(Projected, ScreenVertex), String> {
                let p = projected[raw[4 + c] as usize];
                let uv = texture.map_or([0.0; 2], |(_, t)| {
                    [
                        (f32::from(raw[20 + 2 * c]) + 0.5) / f32::from(t.width),
                        (f32::from(raw[21 + 2 * c]) + 0.5) / f32::from(t.height),
                    ]
                });
                Ok((
                    p,
                    ScreenVertex {
                        x: p.sx,
                        y: p.sy,
                        z: p.z,
                        rhw: p.hz,
                        diffuse: colour(c)?,
                        uv,
                    },
                ))
            })
            .collect::<Result<_, _>>()?;
        let and = corners.iter().fold(0x3f, |a, (p, _)| a & p.outcode);
        let or = corners.iter().fold(0, |a, (p, _)| a | p.outcode);
        if and & 0x10 != 0 || (and & 0x3f != 0 && or & 0x10 == 0) {
            return Ok(());
        }
        let records: Vec<Vec<ScreenVertex>> = if texture.is_some() && count == 4 {
            let v: Vec<ScreenVertex> = corners.iter().map(|c| c.1).collect();
            vec![vec![v[0], v[1], v[3]], vec![v[3], v[1], v[2]]]
        } else {
            vec![corners.iter().map(|c| c.1).collect()]
        };
        for mut v in records {
            if or & 0x10 != 0 {
                match self.clip_near(&v) {
                    Some(clipped) => v = clipped,
                    None => continue,
                }
            }
            if back_facing(&v, ctx.mirrored) {
                continue;
            }
            let (s, use_min) = match texture {
                _ if ctx.figure => (0.7, false),
                Some((_, t)) if f & 0x40 == 0 && t.opaque => (1.0, false),
                Some(_) => (0.95, bias == 0.0),
                None if f & 0x40 == 0 => (1.0, false),
                None => (0.95, bias == 0.0),
            };
            let near = self.viewport.near as f32;
            let mut key = if use_min { f32::MAX } else { f32::MIN };
            for vertex in &mut v {
                let zb = (bias + vertex.z) * s;
                key = if use_min { key.min(zb) } else { key.max(zb) };
                vertex.z = (near / zb.max(near)).min(Z_CAP);
            }
            let slot = (((key + 0.5) as i32).max(0) >> 2).min(SLOTS);
            let mut flags = f;
            // half transparent faces on textures with clear texels draw opaque instead
            if let Some((_, t)) = texture
                && f & 0x1c0 == 0x40
                && !t.opaque
            {
                flags = (flags & !0x40) | 0x80;
            }
            polys.push(Poly {
                fogged: true,
                item: 0,
                face: 0,
                flags,
                texture: texture.map(|t| t.0),
                vertices: v,
                slot,
            });
        }
        Ok(())
    }
}

impl View {
    /// Cuts a polygon at the near plane in view space and projects it again, up to six corners.
    fn clip_near(&self, v: &[ScreenVertex]) -> Option<Vec<ScreenVertex>> {
        let (h, near) = (self.h as f32, self.viewport.near as f32);
        let (cx, cy) = (
            self.viewport.width as f32 * 0.5,
            self.viewport.height as f32 * 0.5,
        );
        let unprojected: Vec<[f32; 3]> = v
            .iter()
            .map(|p| {
                let q = 1.0 / p.rhw;
                [(p.x - cx) * q, (p.y - cy) * q, h * q]
            })
            .collect();
        let mut out: Vec<([f32; 3], u32, [f32; 2])> = Vec::with_capacity(6);
        let n = v.len();
        for a in 0..n {
            let b = (a + 1) % n;
            let (pa, pb) = (unprojected[a], unprojected[b]);
            let (ina, inb) = (pa[2] >= near, pb[2] >= near);
            if ina {
                out.push((pa, v[a].diffuse, v[a].uv));
            }
            if ina != inb {
                let t = (near - pa[2]) / (pb[2] - pa[2]);
                let lerp = |x: f32, y: f32| (y - x) * t + x;
                let byte = |s: u32| {
                    let (ca, cb) = (
                        (v[a].diffuse >> s & 0xff) as i32,
                        (v[b].diffuse >> s & 0xff) as i32,
                    );
                    ((ca + ((cb - ca) as f32 * t) as i32) as u32 & 0xff) << s
                };
                let diffuse = (v[a].diffuse & 0xff00_0000) | byte(16) | byte(8) | byte(0);
                let uv = [lerp(v[a].uv[0], v[b].uv[0]), lerp(v[a].uv[1], v[b].uv[1])];
                out.push(([lerp(pa[0], pb[0]), lerp(pa[1], pb[1]), near], diffuse, uv));
            }
        }
        if !(3..=6).contains(&out.len()) {
            return None;
        }
        let clipped: Vec<ScreenVertex> = out
            .into_iter()
            .map(|(p, diffuse, uv)| {
                let hz = h / p[2];
                ScreenVertex {
                    x: hz * p[0] + cx,
                    y: p[1] * hz + cy,
                    z: p[2],
                    rhw: hz,
                    diffuse,
                    uv,
                }
            })
            .collect();
        let and = clipped
            .iter()
            .fold(0xf, |a, p| a & (self.outcode(p.x, p.y, p.z) & 0xf));
        (and == 0).then_some(clipped)
    }
}

/// Clockwise on screen is front facing, the other way round when mirrored, and a quad whose two
/// halves disagree is kept.
fn back_facing(v: &[ScreenVertex], mirrored: bool) -> bool {
    let n = v.len();
    let c1 = (v[n - 1].y - v[0].y) * (v[1].x - v[0].x) - (v[n - 1].x - v[0].x) * (v[1].y - v[0].y);
    if n >= 4 {
        let c2 = (v[3].x - v[2].x) * (v[1].y - v[2].y) - (v[3].y - v[2].y) * (v[1].x - v[2].x);
        if (c2 <= 0.0) != (c1 <= 0.0) {
            return false;
        }
    }
    if mirrored { c1 > 0.0 } else { c1 <= 0.0 }
}

impl View {
    /// The model an item draws at its view depth, following the level of detail chain, or none
    /// for a model marked hidden.
    fn lod<'s>(
        &self,
        scene: &'s Scene,
        mut index: usize,
        rel: [i32; 3],
    ) -> Result<Option<&'s Model>, String> {
        let v = rel.map(|c| i64::from(c as i16));
        let r = &self.rows;
        let viewz = ((i64::from(r[6]) * v[0] + i64::from(r[7]) * v[1] + i64::from(r[8]) * v[2])
            >> 12) as i32;
        loop {
            let model = scene
                .models
                .get(index)
                .ok_or_else(|| format!("[render] model={index} missing"))?;
            let word = |at: usize| u16::from_le_bytes([model.header[at], model.header[at + 1]]);
            if word(0) & 0x20 != 0 {
                return Ok(None);
            }
            let (lod, next) = (word(0x18) as i16, word(0x1a));
            if next == 0xffff {
                return Ok(Some(model));
            }
            let reach = i32::from(lod) * self.h / 0xbf;
            // negative distances count in 1024ths of the far plane
            let reach = if lod < 0 {
                -((i32::from(reach as i16) * self.viewport.far) >> 10)
            } else {
                reach
            };
            if viewz <= reach {
                return Ok(Some(model));
            }
            index = usize::from(next);
        }
    }
}

fn check_item(index: usize, item: &Item, model: &Model) -> Result<(), String> {
    let model_flags = u16::from_le_bytes([model.header[0], model.header[1]]);
    if item.rotation != [0; 3] {
        return Err(format!(
            "[render] item={index} rotation={:?} unsupported",
            item.rotation
        ));
    }
    if item.flags[0] & !OPEN_FAR != 0 || item.flags[1] != 0 {
        return Err(format!(
            "[render] item={index} flags={:04x?} unsupported",
            item.flags
        ));
    }
    if model_flags & 0x604 != 0 {
        return Err(format!(
            "[render] item={index} model flags={model_flags:#x} unsupported"
        ));
    }
    Ok(())
}

/// The ordering table walked from the farthest slot, newest polygon first within a slot.
pub fn draw_order(polys: &[Poly]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..polys.len()).collect();
    order.sort_by(|&a, &b| polys[b].slot.cmp(&polys[a].slot).then(b.cmp(&a)));
    order
}

/// A draw call as the polygon handler issues it.
#[derive(Clone, Debug, PartialEq)]
pub struct Draw {
    pub item: usize,
    pub face: usize,
    pub slot: i32,
    pub vertices: Vec<ScreenVertex>,
    pub texture: Option<u32>,
    /// Blend mode of the ordering table walker: 1 alpha, 2 add, 4 subtract.
    pub blend: u32,
    pub translucent: bool,
    pub wrap: bool,
}

impl Draw {
    /// The device state the polygon handler sets: blend factors from the mode, depth writes only
    /// for opaque faces on opaque textures, texture and diffuse modulated or diffuse alone.
    pub fn state(&self, textures: &[TextureInfo]) -> crate::raster::DrawState {
        use crate::raster::{BLEND_ONE, DrawState};
        let (src, dst) = match self.blend {
            1 => (5, 6),
            2 => (5, BLEND_ONE),
            _ => (1, 4),
        };
        let opaque_texture = self
            .texture
            .is_none_or(|t| textures.get(t as usize).is_some_and(|t| t.opaque));
        let op = if self.texture.is_some() { 4 } else { 3 };
        DrawState {
            blend: true,
            src,
            dst,
            depth_test: true,
            depth_write: !self.translucent && opaque_texture,
            depth_func: 7,
            colour_op: op,
            alpha_op: op,
            wrap: self.wrap,
            gouraud: true,
        }
    }
}

/// Distance fog: the start as a fraction of the far plane and the colour it fades to.
#[derive(Clone, Copy, Debug)]
pub struct Fog {
    pub start: f32,
    pub colour: [u8; 3],
}

impl View {
    pub fn fog(&self, colour: [u8; 3]) -> Fog {
        Fog {
            start: self.viewport.fog_start as f32 / self.viewport.far as f32,
            colour,
        }
    }

    /// Blend alpha, fog and gamma applied to each polygon in table order.
    pub fn draws(&self, polys: &[Poly], fog: Fog, gamma: &[u8; 256]) -> Vec<Draw> {
        let far = self.viewport.far as f32 * self.viewport.fog_scale;
        draw_order(polys)
            .into_iter()
            .filter_map(|i| {
                let p = &polys[i];
                let (blend, alpha) = match (p.flags & 0x40 != 0, p.flags & 0x180) {
                    (false, _) => (1, 0xff),
                    (true, 0) => (1, 0x80),
                    (true, 0x80) => (2, 0xff),
                    (true, 0x100) => (4, 0),
                    (true, _) => (2, 0x40),
                };
                let vertices = p
                    .vertices
                    .iter()
                    .map(|v| {
                        let mut c = [24, 16, 8, 0].map(|s| (v.diffuse >> s) & 0xff);
                        c[0] = alpha;
                        if p.fogged && fog.start < 1.0 {
                            let r = (self.h as f32 / v.rhw) / far;
                            if r >= fog.start {
                                let t = ((r - fog.start) / (1.0 - fog.start)).min(1.0);
                                let step = (t * 255.0) as u32;
                                let tt = f64::from(step) * f64::from(0.003_921_569f32);
                                c[0] = ((1.0 - tt) * f64::from(c[0])) as u32;
                                for (channel, fc) in c[1..].iter_mut().zip(fog.colour) {
                                    let mixed =
                                        tt * f64::from(fc) + (1.0 - tt) * f64::from(*channel);
                                    *channel = (mixed as u32).min(255);
                                }
                            }
                        }
                        if p.flags & 0x1c0 != 0x140 {
                            for channel in &mut c[1..] {
                                *channel = u32::from(gamma[*channel as usize]);
                            }
                        }
                        ScreenVertex {
                            diffuse: c[0] << 24 | c[1] << 16 | c[2] << 8 | c[3],
                            ..*v
                        }
                    })
                    .collect::<Vec<ScreenVertex>>();
                // a polygon fogged out at every corner is not drawn
                if alpha != 0 && vertices.iter().all(|v| v.diffuse >> 24 == 0) {
                    return None;
                }
                Some(Draw {
                    item: p.item,
                    face: p.face,
                    slot: p.slot,
                    vertices,
                    texture: p.texture,
                    blend,
                    translucent: p.flags & 0x40 != 0,
                    wrap: p.flags & 0x20 != 0,
                })
            })
            .collect()
    }
}

/// Hangar's viewport and level settings as the original sets them up, read from it at runtime.
pub const HANGAR_VIEWPORT: Viewport = Viewport {
    width: 640,
    height: 480,
    near: 10,
    far: 6524,
    fov: 3413,
    fog_start: 5500,
    fog_scale: 1.5,
};

/// Hangar's per model depth bias, set by code in the original.
pub fn hangar_bias(model: usize) -> i32 {
    match model {
        420..=422 => 4096,
        33 | 38 | 135 => 128,
        62 | 136 | 137 | 378 => 32,
        400 | 401 => -16,
        _ => 0,
    }
}
