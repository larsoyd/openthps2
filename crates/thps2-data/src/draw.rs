use crate::psx::Face;

#[derive(Clone, Copy, Debug)]
pub struct FaceOverride {
    pub mask: u16,
    pub set: u16,
}

impl Default for FaceOverride {
    fn default() -> Self {
        Self {
            mask: u16::MAX,
            set: 0,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Corner {
    pub vertex: u8,
    pub uv: Option<[f32; 2]>,
}

#[derive(Debug)]
pub struct DrawFace {
    pub flags: u16,
    pub texture_index: Option<u32>,
    pub corners: Vec<Corner>,
}

pub fn prepare_face(
    face: &Face,
    overrides: FaceOverride,
    logical_size: Option<[u16; 2]>,
    split_textured_quads: bool,
) -> Result<Vec<DrawFace>, String> {
    let source = face.flags as u16;
    let initialized = if source & 0x40 != 0 {
        source
    } else {
        source ^ 0x80
    };
    let flags = (initialized & overrides.mask) | overrides.set;
    if flags & 0xc0 == 0 {
        return Ok(Vec::new());
    }
    let triangle = flags & 0x10 != 0;
    let count = if triangle { 3 } else { 4 };
    if face.indices.len() < count {
        return Err(format!(
            "draw flags={flags:#x} corners={} required={count}",
            face.indices.len()
        ));
    }
    let texture_index = if flags & 1 != 0 {
        Some(
            face.texture_index
                .ok_or_else(|| format!("draw flags={flags:#x} missing texture index"))?,
        )
    } else {
        None
    };
    let size = if let Some(index) = texture_index {
        let size = logical_size
            .filter(|s| s[0] != 0 && s[1] != 0)
            .ok_or_else(|| format!("draw texture={index} invalid logical size={logical_size:?}"))?;
        if face.raw.len() < 20 + count * 2 {
            return Err(format!(
                "draw texture={index} truncated UV bytes={} corners={count}",
                face.raw.len()
            ));
        }
        Some(size)
    } else {
        None
    };
    let orders: &[&[usize]] = if triangle {
        &[&[0, 1, 2]]
    } else if texture_index.is_some() && split_textured_quads {
        &[&[0, 1, 2], &[2, 1, 3]]
    } else {
        &[&[0, 1, 3, 2]]
    };
    Ok(orders
        .iter()
        .map(|order| DrawFace {
            flags,
            texture_index,
            corners: order
                .iter()
                .map(|&i| Corner {
                    vertex: face.indices[i],
                    uv: size.map(|s| {
                        [
                            (face.raw[20 + i * 2] as f32 + 0.5) / s[0] as f32,
                            (face.raw[21 + i * 2] as f32 + 0.5) / s[1] as f32,
                        ]
                    }),
                })
                .collect(),
        })
        .collect())
}

#[derive(Debug, PartialEq)]
pub enum PreparedVertex {
    Position([i16; 3]),
    ScreenOffset { anchors: [usize; 2], distance: i16 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectedVertex {
    /// Native screen pixels, with positive Y downward.
    pub pixel: [f32; 2],
    /// Camera-space Z in source units.
    pub depth: f32,
    /// Native focal length divided by camera-space Z.
    pub scale: f32,
}

pub fn prepare_vertices(vertices: &[crate::psx::Vertex]) -> Result<Vec<PreparedVertex>, String> {
    vertices
        .iter()
        .enumerate()
        .map(|(index, vertex)| match vertex.flags {
            0 => Ok(PreparedVertex::Position(vertex.words)),
            16 => {
                let anchors = [
                    (vertex.words[0] as u16 >> 3) as usize,
                    (vertex.words[1] as u16 >> 3) as usize,
                ];
                if anchors.iter().any(|&anchor| anchor >= index) {
                    return Err(format!(
                        "vertex={index} offset anchors={anchors:?} must precede vertex"
                    ));
                }
                Ok(PreparedVertex::ScreenOffset {
                    anchors,
                    distance: vertex.words[2],
                })
            }
            flags => Err(format!("vertex={index} unsupported flags={flags:#x}")),
        })
        .collect()
}

impl PreparedVertex {
    pub fn resolve_offset(&self, projected: &[ProjectedVertex]) -> Result<ProjectedVertex, String> {
        let Self::ScreenOffset { anchors, distance } = self else {
            return Err("offset resolution requires a screen-offset vertex".into());
        };
        let get = |index: usize| {
            projected
                .get(index)
                .copied()
                .ok_or_else(|| format!("offset anchor={index} projected_count={}", projected.len()))
        };
        let a = get(anchors[0])?;
        let b = get(anchors[1])?;
        if [
            a.pixel[0], a.pixel[1], a.depth, a.scale, b.pixel[0], b.pixel[1],
        ]
        .iter()
        .any(|v| !v.is_finite())
        {
            return Err(format!("offset anchors={anchors:?} non-finite projection"));
        }
        let dx = b.pixel[0] - a.pixel[0];
        let dy = -(b.pixel[1] - a.pixel[1]);
        let length = (dx * dx + dy * dy).sqrt();
        if length == 0.0 || !length.is_finite() {
            return Err(format!(
                "offset anchors={anchors:?} coincident or overflowing projection"
            ));
        }
        let pixel = [
            a.pixel[0] + (dy / length) * *distance as f32 * a.scale,
            a.pixel[1] + (dx / length) * *distance as f32 * a.scale,
        ];
        if pixel.iter().any(|v| !v.is_finite()) {
            return Err(format!("offset anchors={anchors:?} non-finite result"));
        }
        Ok(ProjectedVertex {
            pixel,
            depth: a.depth,
            scale: a.scale,
        })
    }
}
