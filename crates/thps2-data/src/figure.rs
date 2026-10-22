use crate::{
    pose::{PoseRecord, Rig},
    psx::Scene,
    render::{Faces, Poly, Projected, TextureInfo, View},
};

const SHARED: usize = 256;
const Q12: f32 = 1.0 / 4096.0;
const Q16: f32 = 1.0 / 65536.0;

/// The skater's light: directions, the colour each direction gives and the ambient colour, 4096 = 1.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Light {
    pub directions: [i16; 9],
    pub colours: [i16; 9],
    pub ambient: [i32; 3],
}

impl Light {
    /// The 0x34 byte light record: directions, a pad word, colours, a pad word, then ambient.
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 0x34 {
            return Err(format!("[figure] light bytes={} need=52", bytes.len()));
        }
        let half = |at: usize| i16::from_le_bytes([bytes[at], bytes[at + 1]]);
        Ok(Self {
            directions: std::array::from_fn(|k| half(2 * k)),
            colours: std::array::from_fn(|k| half(0x14 + 2 * k)),
            ambient: std::array::from_fn(|k| {
                i32::from_le_bytes(bytes[0x28 + 4 * k..0x2c + 4 * k].try_into().unwrap())
            }),
        })
    }
}

/// Hangar's light before the ground scales it, and the scale factors in percent for the three
/// colour columns and the ambient term.
pub const HANGAR_LIGHT: Light = Light {
    directions: [0, 4096, 0, 0, -4096, 0, 4017, 0, -799],
    colours: [1764, 4864, 1536, 1858, 4654, 1536, 2048, 4444, 1536],
    ambient: [1344, 1344, 1536],
};
pub const HANGAR_LIGHT_SCALE: [i32; 4] = [75, 60, 100, 10];

impl Light {
    /// The light under a ground brightness, 0x100 leaving the base light as it is.
    pub fn scaled(&self, scale: [i32; 4], brightness: i32) -> Self {
        let s = |k: usize| scale[k] * (brightness - 0x100) / 100 + 0x100;
        Self {
            directions: self.directions,
            colours: std::array::from_fn(|k| ((i32::from(self.colours[k]) * s(k % 3)) >> 8) as i16),
            ambient: std::array::from_fn(|i| (self.ambient[i] * s(3)) >> 8),
        }
    }
}

/// The physics basis tilted by the manual balance, as the skater is drawn.
pub fn tilted_basis(basis: [i32; 9], balance: i32) -> [i16; 9] {
    let m = basis.map(|v| v as i16);
    if balance == 0 {
        return m;
    }
    let a = ((balance / 10) * 80 / 100) & 0xfff;
    mvmva(&m, &crate::pose::euler([a as i16, 0, 0]))
}

/// The drawing matrix of a basis: its up column turned over.
pub fn object_matrix(basis: [i16; 9]) -> [i16; 9] {
    let mut m = basis;
    for k in [1, 4, 7] {
        m[k] = m[k].wrapping_neg();
    }
    m
}

/// The basis pressed flat onto the ground plane: every height comes from the plane through the
/// ground point instead. A level normal without height counts as one unit.
pub fn shadow_matrix(basis: [i16; 9], normal: [i16; 3]) -> [i16; 9] {
    let ny = if normal[1] == 0 {
        1
    } else {
        i32::from(normal[1])
    };
    let a = ((-i32::from(normal[0]) << 12) / ny) as i16;
    let b = ((-i32::from(normal[2]) << 12) / ny) as i16;
    object_matrix(mvmva(&[4096, 0, 0, a, 0, b, 0, 0, 4096], &basis))
}

/// The grey the shadow draws in, darker the brighter the ground light.
pub fn shadow_colour(brightness: i32) -> u32 {
    let f = (f64::from(brightness) * f64::from(1.0f32 / 255.0)).clamp(0.0, 1.0);
    let v = ((1.0 - f) as f32 * 10.2f32) as u32;
    0xff00_0000 | (v * 0x0001_0101)
}

/// Part names of a model header file, as the defines list them, without the skater prefix.
fn part_names(header: &str) -> Result<Vec<String>, String> {
    let mut names = Vec::new();
    for line in header.lines().filter(|l| l.starts_with("#define ")) {
        let mut words = line.split_whitespace().skip(1);
        let (Some(name), Some(index)) = (words.next(), words.next()) else {
            continue;
        };
        let Some((_, part)) = name.split_once("PART_") else {
            continue;
        };
        let index: usize = index
            .parse()
            .map_err(|_| format!("[figure] part define={name} index={index}"))?;
        if index != names.len() {
            return Err(format!("[figure] part define={name} out of order"));
        }
        let suffix = part.split_once('_').map_or(part, |(_, rest)| rest);
        names.push(suffix.to_string());
    }
    Ok(names)
}

/// A skater model drawn with another bank's animation: its parts, the pose each part takes and
/// the shared vertices and lights the parts hand each other.
pub struct Figure {
    pub scene: Scene,
    pub rig: Rig,
    /// The pose record of each model part, matched by part name.
    pub remap: Vec<usize>,
    pub textures: Vec<TextureInfo>,
    /// Model parts of the board and its wheels, as bits by part index.
    pub board: u32,
}

/// Screen records and lights exported by parts for later parts, kept between draws like the
/// original's buffers, so a part that reads an index no part has written this draw sees an old one.
#[derive(Clone, Debug)]
pub struct Shared {
    vertices: Vec<Projected>,
    lights: Vec<u32>,
}

impl Default for Shared {
    fn default() -> Self {
        Self {
            vertices: vec![Projected::default(); SHARED],
            lights: vec![0; SHARED],
        }
    }
}

impl Figure {
    pub fn new(
        scene: Scene,
        rig: Rig,
        model_header: &str,
        rig_header: &str,
        textures: Vec<TextureInfo>,
    ) -> Result<Self, String> {
        let (parts, poses) = (part_names(model_header)?, part_names(rig_header)?);
        if parts.len() != scene.models.len() {
            return Err(format!(
                "[figure] names={} models={}",
                parts.len(),
                scene.models.len()
            ));
        }
        let board = ["BOARD", "FRONT_WHEEL", "BACK_WHEEL"]
            .iter()
            .map(|name| {
                parts
                    .iter()
                    .position(|p| p == name)
                    .map(|i| 1 << i)
                    .ok_or_else(|| format!("[figure] no part={name}"))
            })
            .sum::<Result<u32, String>>()?;
        let remap = parts
            .iter()
            .map(|p| {
                poses
                    .iter()
                    .position(|q| q == p)
                    .ok_or_else(|| format!("[figure] part={p} has no pose"))
            })
            .collect::<Result<_, _>>()?;
        Ok(Self {
            scene,
            rig,
            remap,
            textures,
            board,
        })
    }
}

fn sat16(v: i32) -> i16 {
    v.clamp(-32768, 32767) as i16
}

/// a times the columns of b through the transform unit, each entry saturated.
pub(crate) fn mvmva(a: &[i16; 9], b: &[i16; 9]) -> [i16; 9] {
    std::array::from_fn(|k| {
        let (i, j) = (k / 3, k % 3);
        let sum: i32 = (0..3)
            .map(|n| i32::from(a[3 * i + n]) * i32::from(b[3 * n + j]))
            .sum();
        sat16(sum >> 12)
    })
}

/// Vertex light from its normal: directions clamped at zero through the colour matrix plus the
/// ambient term. Two of three terms are stored as single floats on the way, the third is not.
fn light(directions: &[i16; 9], light: &Light, normal: [i16; 3]) -> u32 {
    let l = |k: usize| f64::from(directions[k]) / 4096.0;
    let c = |k: usize| f64::from(light.colours[k]) / 4096.0;
    let n = normal.map(|v| f64::from(v) / 4096.0);
    let d = |i: usize| ((l(3 * i + 2) * n[2] + l(3 * i + 1) * n[1]) + l(3 * i) * n[0]).max(0.0);
    let (d0, d1, d2) = (f64::from(d(0) as f32), f64::from(d(1) as f32), d(2));
    let channel = |j: usize| {
        ((c(3 * j + 2) * d2 + c(3 * j + 1) * d1) + c(3 * j) * d0)
            + f64::from(light.ambient[j]) / 4096.0
    };
    let (c0, c1, c2) = (
        f64::from(channel(0) as f32),
        f64::from(channel(1) as f32),
        channel(2),
    );
    let byte = |v: f64| (v.min(1.0) * 255.0) as u32;
    byte(c0) << 16 | byte(c1) << 8 | byte(c2)
}

/// The skater in one frame: the pose records, the object matrix and position in 20.12 fixed
/// point, its light, whether it rides switch and which parts are hidden, as bits by part.
pub struct Placement<'a> {
    pub pose: &'a [PoseRecord],
    pub orientation: [i16; 9],
    pub position: [i32; 3],
    pub light: Light,
    pub mirrored: bool,
    pub hidden: u32,
    /// The shadow pass: one flat colour, no light and no textures.
    pub shadow: Option<u32>,
}

impl View {
    /// The skater's parts in model order: each part placed by its pose record, lit as if it had
    /// the object's orientation, its faces in the ordering table at seven tenths of their depth.
    pub fn figure(
        &self,
        figure: &Figure,
        place: &Placement,
        shared: &mut Shared,
    ) -> Result<Vec<Poly>, String> {
        let (pose, position, light_in, mirrored, hidden) = (
            place.pose,
            place.position,
            &place.light,
            place.mirrored,
            place.hidden,
        );
        let mut orientation = place.orientation;
        if mirrored {
            for k in [0, 3, 6] {
                orientation[k] = orientation[k].wrapping_neg();
            }
        }
        let orientation = &orientation;
        let c = mvmva(&self.rows, orientation);
        let rel: [i32; 3] =
            std::array::from_fn(|i| i32::from(((position[i] >> 12) - self.eye[i]) as i16));
        let t: [i32; 3] = std::array::from_fn(|i| {
            (0..3)
                .map(|n| i32::from(self.rows[3 * i + n]) * rel[n])
                .sum::<i32>()
                >> 12
        });
        let directions = mvmva(&light_in.directions, orientation);
        let cf: [f32; 9] = c.map(|v| f32::from(v) * Q12);
        let (mut exported, mut lit) = (0, 0);
        let mut polys = Vec::new();
        for (part, model) in figure.scene.models.iter().enumerate() {
            let record = pose
                .get(figure.remap[part])
                .ok_or_else(|| format!("[figure] part={part} pose missing"))?;
            let pf: [f32; 9] = record.m.map(|v| f32::from(v) * Q16);
            let tp = record.t.map(|v| f32::from(v) * 0.0625);
            let rf: [[f32; 3]; 3] = std::array::from_fn(|i| {
                std::array::from_fn(|j| {
                    (cf[3 * i] * pf[j] + cf[3 * i + 2] * pf[6 + j]) + cf[3 * i + 1] * pf[3 + j]
                })
            });
            let tf: [f32; 3] = std::array::from_fn(|i| {
                ((cf[3 * i + 2] * tp[2] + cf[3 * i + 1] * tp[1]) + cf[3 * i] * tp[0]) + t[i] as f32
            });
            if part == 0 {
                (exported, lit) = (0, 0);
            }
            let mut projected = Vec::with_capacity(model.vertices.len());
            for (vi, v) in model.vertices.iter().enumerate() {
                let p = match v.flags {
                    0 | 1 => self.project(&rf, tf, v.words),
                    2 => *shared.vertices.get(v.words[1] as usize).ok_or_else(|| {
                        format!("[figure] part={part} vertex={vi} shared={}", v.words[1])
                    })?,
                    f => {
                        return Err(format!(
                            "[figure] part={part} vertex={vi} flags={f:#x} unsupported"
                        ));
                    }
                };
                if v.flags == 1 {
                    *shared
                        .vertices
                        .get_mut(exported)
                        .ok_or_else(|| format!("[figure] part={part} exports past {SHARED}"))? = p;
                    exported += 1;
                }
                projected.push(p);
            }
            let mut lights = Vec::with_capacity(model.normals_raw.len());
            for (k, raw) in model
                .normals_raw
                .iter()
                .enumerate()
                .take(if place.shadow.is_some() {
                    0
                } else {
                    usize::MAX
                })
            {
                let normal = [0, 2, 4].map(|at| i16::from_le_bytes([raw[at], raw[at + 1]]));
                let flags = model.vertices.get(k).map_or(0, |v| v.flags);
                let value = match flags {
                    2 => shared.lights[model.vertices[k].words[1] as usize],
                    _ => light(&directions, light_in, normal),
                };
                if flags == 1 {
                    shared.lights[lit] = value;
                    lit += 1;
                }
                lights.push(value);
            }
            let faces = Faces {
                textures: &figure.textures,
                colours: &[],
                lights: Some(&lights),
                bias: 0.0,
                figure: true,
                mirrored,
                flat: place.shadow,
                clear: 0,
            };
            let invisible = u16::from_le_bytes([model.header[0], model.header[1]]) & 0x20 != 0;
            // a hidden part still hands its vertices and lights on, it only draws no faces
            if hidden >> part & 1 != 0 || (place.shadow.is_some() && invisible) {
                continue;
            }
            for (fi, face) in model.faces.iter().enumerate() {
                let at = polys.len();
                self.face(&faces, &projected, face, &mut polys)
                    .map_err(|e| format!("[figure] part={part} face={fi} {e}"))?;
                for p in &mut polys[at..] {
                    (p.item, p.face) = (part, fi);
                }
            }
        }
        Ok(polys)
    }
}
