use crate::psx::Scene;

#[derive(Debug)]
pub struct ZoneCell {
    pub header: [i32; 2],
    pub items: Vec<usize>,
}

#[derive(Debug)]
pub struct ZoneGrid {
    /// XZ bounds in source Q12 units.
    pub min: [i32; 2],
    pub max: [i32; 2],
    pub dimensions: [usize; 2],
    pub cells: Vec<ZoneCell>,
    cell_size: i64,
}

impl ZoneGrid {
    pub fn parse(data: &[u8], item_count: usize) -> Result<Self, String> {
        let word = |offset: usize| -> Result<i32, String> {
            let bytes = data
                .get(offset..offset.saturating_add(4))
                .ok_or_else(|| format!("zone offset={offset} truncated word"))?;
            Ok(i32::from_le_bytes(bytes.try_into().unwrap()))
        };
        let min = [word(0)?, word(4)?];
        let max = [word(8)?, word(12)?];
        let packed = word(16)? as u32;
        let dimensions = [(packed & 0xffff) as usize, (packed >> 16) as usize];
        if dimensions.iter().any(|&v| v == 0 || v > 20) || (0..2).any(|axis| min[axis] >= max[axis])
        {
            return Err("zone invalid dimensions or bounds".into());
        }
        let cell_size = (i64::from(max[0]) - i64::from(min[0])) / dimensions[0] as i64;
        if cell_size == 0
            || (i64::from(max[1]) - i64::from(min[1])) / dimensions[1] as i64 != cell_size
        {
            return Err("zone unequal or empty cell extents unsupported".into());
        }
        let mut offset = 20;
        let mut cells = Vec::with_capacity(dimensions[0] * dimensions[1]);
        for index in 0..dimensions[0] * dimensions[1] {
            let header = [word(offset)?, word(offset + 4)?];
            let count = word(offset + 8)?;
            offset += 12;
            if count < 0 || count as usize > data.len().saturating_sub(offset) / 4 {
                return Err(format!("zone cell={index} invalid item count={count}"));
            }
            let mut items = Vec::with_capacity(count as usize);
            for _ in 0..count {
                let item = word(offset)?;
                if item < 0 || item as usize >= item_count {
                    return Err(format!("zone cell={index} invalid item={item}"));
                }
                items.push(item as usize);
                offset += 4;
            }
            if word(offset)? != 0 {
                return Err(format!("zone cell={index} nonzero terminator"));
            }
            offset += 4;
            cells.push(ZoneCell { header, items });
        }
        if offset != data.len() {
            return Err(format!("zone trailing bytes={}", data.len() - offset));
        }
        Ok(Self {
            min,
            max,
            dimensions,
            cells,
            cell_size,
        })
    }

    pub fn cell_at(&self, xz: [i32; 2]) -> Option<[usize; 2]> {
        if (0..2).any(|a| xz[a] < self.min[a] || xz[a] > self.max[a]) {
            return None;
        }
        Some(std::array::from_fn(|a| {
            (((i64::from(xz[a]) - i64::from(self.min[a])) / self.cell_size) as usize)
                .min(self.dimensions[a] - 1)
        }))
    }

    pub fn cell(&self, coordinates: [usize; 2]) -> Option<&ZoneCell> {
        if (0..2).any(|a| coordinates[a] >= self.dimensions[a]) {
            return None;
        }
        self.cells
            .get(coordinates[1] * self.dimensions[0] + coordinates[0])
    }
}

#[derive(Debug)]
struct CollisionFace {
    index: usize,
    vertices: Vec<[i16; 3]>,
    normal: [i16; 3],
    bounds: [[i64; 3]; 2],
    flags: u32,
}

#[derive(Debug)]
struct CollisionItem {
    position: [i32; 3],
    model: usize,
    faces: Vec<CollisionFace>,
}

#[derive(Debug)]
pub struct CollisionWorld {
    pub zone: ZoneGrid,
    items: Vec<CollisionItem>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hit {
    pub item_index: usize,
    pub model_index: usize,
    pub face_index: usize,
    pub t_q14: i32,
    pub position_fixed: [i32; 3],
    /// Source Q12 normal, with negative Y pointing upward.
    pub normal: [i16; 3],
    pub distance_units: i32,
    pub flags: u32,
}

impl CollisionWorld {
    pub fn from_scene(scene: &Scene) -> Result<Self, String> {
        let mut zones = scene.chunks.iter().filter(|c| c.kind == 10);
        let zone = ZoneGrid::parse(
            &zones
                .next()
                .ok_or("scene missing collision zone chunk10")?
                .payload,
            scene.items.len(),
        )?;
        if zones.next().is_some() {
            return Err("scene has multiple collision zone chunks".into());
        }
        let mut items = Vec::with_capacity(scene.items.len());
        for (item_index, item) in scene.items.iter().enumerate() {
            if item.rotation != [0; 3] {
                return Err(format!(
                    "collision item={item_index} rotated items unsupported"
                ));
            }
            let model = scene
                .models
                .get(item.model_index as usize)
                .ok_or("collision invalid model index")?;
            let mut faces = Vec::new();
            for (index, face) in model.faces.iter().enumerate() {
                let raw = face
                    .raw
                    .get(12..16)
                    .ok_or("collision truncated face fields")?;
                let flags = u32::from_le_bytes(raw.try_into().unwrap());
                let normal_raw = model
                    .normals_raw
                    .get((flags & 0xffff) as usize)
                    .ok_or_else(|| {
                        format!("collision item={item_index} face={index} invalid normal index")
                    })?;
                if flags & 0x30000 == 0x10000 || flags & 0x20000 != 0 || flags & 0x400000 != 0 {
                    continue;
                }
                let normal = std::array::from_fn(|a| {
                    i16::from_le_bytes([normal_raw[a * 2], normal_raw[a * 2 + 1]])
                });
                let order: &[usize] = if face.flags & 0x10 != 0 {
                    &[0, 1, 2]
                } else {
                    &[0, 1, 3, 2]
                };
                let mut vertices = Vec::with_capacity(order.len());
                for &corner in order {
                    let vertex_index = *face
                        .indices
                        .get(corner)
                        .ok_or("collision missing face corner")?
                        as usize;
                    let vertex = model
                        .vertices
                        .get(vertex_index)
                        .ok_or("collision invalid vertex index")?;
                    // Native collision reads raw XYZ even for renderer screen-offset records.
                    vertices.push(vertex.words);
                }
                let bounds = [
                    std::array::from_fn(|a| {
                        i64::from(vertices.iter().map(|v| v[a]).min().unwrap()) * 4096
                            + i64::from(item.position_fixed[a])
                    }),
                    std::array::from_fn(|a| {
                        i64::from(vertices.iter().map(|v| v[a]).max().unwrap()) * 4096
                            + i64::from(item.position_fixed[a])
                    }),
                ];
                if bounds.iter().flatten().any(|&n| i32::try_from(n).is_err()) {
                    return Err("collision world bounds overflow".into());
                }
                faces.push(CollisionFace {
                    index,
                    vertices,
                    normal,
                    bounds,
                    flags,
                });
            }
            items.push(CollisionItem {
                position: item.position_fixed,
                model: item.model_index as usize,
                faces,
            });
        }
        Ok(Self { zone, items })
    }

    /// Q12 endpoints, static skater filter, no trigger execution or moving objects.
    pub fn query_single_cell(&self, start: [i32; 3], end: [i32; 3]) -> Result<Option<Hit>, String> {
        if start == end {
            return Ok(None);
        }
        if [0, 2].into_iter().enumerate().any(|(i, a)| {
            start[a].max(end[a]) < self.zone.min[i] || start[a].min(end[a]) > self.zone.max[i]
        }) {
            return Ok(None);
        }
        let start_cell = self.zone.cell_at([start[0], start[2]]);
        let end_cell = self.zone.cell_at([end[0], end[2]]);
        let Some(cell) = start_cell.filter(|_| start_cell == end_cell) else {
            return Err("collision multi-cell/outside-to-inside traversal unsupported".into());
        };
        let delta: [i64; 3] = std::array::from_fn(|a| i64::from(end[a]) - i64::from(start[a]));
        if delta.iter().any(|&v| i32::try_from(v).is_err()) {
            return Err("collision query delta overflow".into());
        }
        let units = delta.map(|v| v >> 12);
        let length = if units[0] == 0 && units[2] == 0 {
            units[1].abs()
        } else {
            units
                .iter()
                .map(|&v| (v as f64).powi(2))
                .sum::<f64>()
                .sqrt() as i64
        };
        if length == 0 {
            return Ok(None);
        }
        let mut best: Option<Hit> = None;
        for &item_index in &self.zone.cell(cell).unwrap().items {
            let item = &self.items[item_index];
            let s: [i64; 3] = std::array::from_fn(|a| {
                i64::from(start[a] >> 12) - i64::from(item.position[a] >> 12)
            });
            let e: [i64; 3] = std::array::from_fn(|a| {
                i64::from(end[a] >> 12) - i64::from(item.position[a] >> 12)
            });
            for face in &item.faces {
                if (0..3).any(|a| {
                    face.bounds[0][a] > i64::from(start[a].max(end[a]) & !0xfff) + 0x4000
                        || face.bounds[1][a] < i64::from(start[a].min(end[a]) & !0xfff) - 0x4000
                }) {
                    continue;
                }
                let plane = |p: [i64; 3]| -> Result<i64, String> {
                    let distance: i64 = (0..3)
                        .map(|a| {
                            i64::from(face.normal[a]) * (p[a] - i64::from(face.vertices[0][a]))
                        })
                        .sum();
                    i32::try_from(distance)
                        .map(i64::from)
                        .map_err(|_| "collision plane distance overflow".into())
                };
                let d0 = plane(s)?;
                let d1 = plane(e)?;
                if d0 < 0 || d1 > 0 || (d0 < 0x800 && d1 > -0x800) {
                    continue;
                }
                let direction: [f64; 3] = std::array::from_fn(|a| (e[a] - s[a]) as f64);
                let mut inside = true;
                for edge in 0..face.vertices.len() {
                    let a: [f64; 3] =
                        std::array::from_fn(|i| (i64::from(face.vertices[edge][i]) - s[i]) as f64);
                    let b: [f64; 3] = std::array::from_fn(|i| {
                        (i64::from(face.vertices[(edge + 1) % face.vertices.len()][i]) - s[i])
                            as f64
                    });
                    let cross = [
                        a[1] * b[2] - a[2] * b[1],
                        a[2] * b[0] - a[0] * b[2],
                        a[0] * b[1] - a[1] * b[0],
                    ];
                    if (0..3).map(|i| cross[i] * direction[i]).sum::<f64>() < 0.0 {
                        inside = false;
                        break;
                    }
                }
                if !inside {
                    continue;
                }
                let denominator =
                    i32::try_from(d0 - d1).map_err(|_| "collision plane difference overflow")?;
                let t = d0 * 0x4000 / i64::from(denominator);
                if best.is_some_and(|hit| i64::from(hit.t_q14) <= t) {
                    continue;
                }
                let position_fixed =
                    std::array::from_fn(|a| (i64::from(start[a]) + delta[a] * t / 0x4000) as i32);
                best = Some(Hit {
                    item_index,
                    model_index: item.model,
                    face_index: face.index,
                    t_q14: t as i32,
                    position_fixed,
                    normal: face.normal,
                    distance_units: (length * t / 0x4000) as i32,
                    flags: face.flags,
                });
            }
        }
        Ok(best)
    }
}
