use std::sync::Arc;

use crate::psx::Scene;
use crate::rng::Rng;

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

    /// Cells a Q12 XZ segment visits in native order, clipped to the zone box first.
    pub fn walk(&self, start: [i32; 2], end: [i32; 2]) -> Vec<[usize; 2]> {
        let [mut sx, mut sz] = start.map(i64::from);
        let [mut ex, mut ez] = end.map(i64::from);
        let [min_x, min_z] = self.min.map(i64::from);
        let [max_x, max_z] = self.max.map(i64::from);
        if (sx < min_x && ex < min_x)
            || (sx > max_x && ex > max_x)
            || (sz < min_z && ez < min_z)
            || (sz > max_z && ez > max_z)
        {
            return Vec::new();
        }
        let cell = self.cell_size;
        let dims = self.dimensions.map(|d| d as i64);
        if sx == ex && sz == ez {
            let cx = ((sx - min_x) / cell).min(dims[0] - 1);
            let cz = ((sz - min_z) / cell).min(dims[1] - 1);
            return vec![[cx as usize, cz as usize]];
        }
        if sx < min_x {
            sz = if sz < ez {
                sz + muldiv(min_x - sx, ez - sz, ex - sx)
            } else {
                ez + muldiv(ex - min_x, sz - ez, ex - sx)
            };
            sx = min_x;
        }
        if ex < min_x {
            ez = if ez < sz {
                ez + muldiv(min_x - ex, sz - ez, sx - ex)
            } else {
                sz + muldiv(sx - min_x, ez - sz, sx - ex)
            };
            ex = min_x;
        }
        if sx > max_x {
            sz = if ez < sz {
                ez + muldiv(max_x - ex, sz - ez, sx - ex)
            } else {
                sz + muldiv(sx - max_x, ez - sz, sx - ex)
            };
            sx = max_x;
        }
        if ex > max_x {
            ez = if sz < ez {
                sz + muldiv(max_x - sx, ez - sz, ex - sx)
            } else {
                ez + muldiv(ex - max_x, sz - ez, ex - sx)
            };
            ex = max_x;
        }
        if sz < min_z {
            sx = if sx < ex {
                sx + muldiv(min_z - sz, ex - sx, ez - sz)
            } else {
                ex + muldiv(ez - min_z, sx - ex, ez - sz)
            };
            sz = min_z;
        }
        if ez < min_z {
            ex = if ex < sx {
                ex + muldiv(min_z - ez, sx - ex, sz - ez)
            } else {
                sx + muldiv(sz - min_z, ex - sx, sz - ez)
            };
            ez = min_z;
        }
        if sz > max_z {
            sx = if ex < sx {
                ex + muldiv(max_z - ez, sx - ex, sz - ez)
            } else {
                sx + muldiv(sz - max_z, ex - sx, sz - ez)
            };
            sz = max_z;
        }
        if ez > max_z {
            ex = if sx < ex {
                sx + muldiv(max_z - sz, ex - sx, ez - sz)
            } else {
                ex + muldiv(ez - max_z, sx - ex, ez - sz)
            };
            ez = max_z;
        }
        let step = |d: i64| if d < 0 { -1 } else { 1 };
        let (dx, step_x) = ((ex - sx).abs(), step(ex - sx));
        let (dz, step_z) = ((ez - sz).abs(), step(ez - sz));
        let (mut cx, mut cz) = ((sx - min_x) / cell, (sz - min_z) / cell);
        let (mut rx, mut rz) = ((sx - min_x) % cell, (sz - min_z) % cell);
        let (mut last_x, mut last_z) = ((ex - min_x) / cell, (ez - min_z) / cell);
        if cx == dims[0] {
            cx -= 1;
            rx += cell;
        }
        if cz == dims[1] {
            cz -= 1;
            rz += cell;
        }
        if last_x == dims[0] {
            last_x -= 1;
        }
        if last_z == dims[1] {
            last_z -= 1;
        }
        let mut to_z = muldiv(dx, rz, cell);
        let mut to_x = muldiv(dz, rx, cell);
        to_x = if sx < ex { to_x - dz } else { -to_x };
        if sz < ez {
            to_z = dx - to_z;
        }
        let mut error = to_z + to_x;
        let mut cells = Vec::new();
        let in_grid = |x: i64, z: i64| x >= 0 && x < dims[0] && z >= 0 && z < dims[1];
        while cx != last_x || cz != last_z {
            if !in_grid(cx, cz) {
                break;
            }
            cells.push([cx as usize, cz as usize]);
            if error < 0 {
                error += dx;
                cz += step_z;
            } else {
                error -= dz;
                cx += step_x;
            }
        }
        if in_grid(cx, cz) {
            cells.push([cx as usize, cz as usize]);
        }
        cells
    }
}

#[derive(Debug)]
struct CollisionFace {
    index: usize,
    vertices: Vec<[i16; 3]>,
    normal: [i16; 3],
    bounds: [[i64; 3]; 2],
    flags: u32,
    face_flags: u32,
}

/// Glass faces stay solid until a kill shatters their item.
const BREAKABLE: u32 = 0x80000;

#[derive(Debug)]
struct CollisionItem {
    position: [i32; 3],
    model: usize,
    faces: Vec<CollisionFace>,
    triggers: Vec<CollisionFace>,
    checksum: u32,
    shatters: bool,
    panes: Vec<Pane>,
}

/// One face of a glass model as a shatter sees it, in model face order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pane {
    breakable: bool,
    textured: bool,
    triangle: bool,
}

/// Faces past the sixth textured one break without shards.
const SHARD_LIMIT: u32 = 6;
const LEAF_DRAWS: [i32; 15] = [
    100, 100, 100, 96, 2, 160, 30, 48, 45, 175, 20, 2, 4096, 4096, 4096,
];
const CENTRED_LEAF: usize = 7;
const PARTICLE_DRAWS: [i32; 14] = [
    4096, 4096, 40, 40, 201, 201, 201, 201, 201, 201, 201, 201, 201, 30,
];

struct Counted<'a> {
    rng: &'a mut Rng,
    draws: u32,
}

impl Counted<'_> {
    fn next(&mut self, n: i32) -> i32 {
        self.draws += 1;
        self.rng.next(n)
    }

    /// A textured shard splits while depth lasts, a leaf on the face's own centre skips one draw.
    fn shard(&mut self, depth: u32, centred: bool) {
        if depth > 0 {
            self.next(9);
            self.shard(depth - 1, false);
            self.shard(depth - 1, false);
            return;
        }
        for (k, &n) in LEAF_DRAWS.iter().enumerate() {
            if !(centred && k == CENTRED_LEAF) {
                self.next(n);
            }
        }
    }
}

/// Draws the random numbers a shatter takes for its shards and particles, returning the count.
pub fn shatter_draws(panes: &[Pane], rng: &mut Rng) -> u32 {
    let mut c = Counted { rng, draws: 0 };
    let mut shards = 0;
    for pane in panes {
        let split = u32::from(c.next(5) == 0);
        if !pane.breakable {
            continue;
        }
        if shards >= SHARD_LIMIT {
            shards += 1;
        } else if pane.textured {
            for _ in 0..if pane.triangle { 1 } else { 2 } {
                c.shard(split, pane.triangle && split == 0);
            }
            shards += 1;
        } else {
            for _ in 0..if pane.triangle { 15 } else { 30 } {
                for n in PARTICLE_DRAWS {
                    c.next(n);
                }
            }
        }
    }
    c.draws
}

/// The level geometry shared by every run, and the items this run has shattered or hidden.
#[derive(Clone, Debug)]
pub struct CollisionWorld {
    pub zone: Arc<ZoneGrid>,
    items: Arc<[CollisionItem]>,
    shattered: Vec<usize>,
    hidden: Vec<bool>,
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
    /// Collision word with the normal index below and the surface bits above 16.
    pub flags: u32,
    /// Face word 0 as stored in the model.
    pub face_flags: u32,
}

/// The level loader flips face word bit 0x80 unless bit 0x40 is set.
fn loaded_face_word(word: u32) -> u32 {
    if word & 0x40 == 0 { word ^ 0x80 } else { word }
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
            let (mut faces, mut triggers) = (Vec::new(), Vec::new());
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
                if flags & 0x30000 == 0x10000 {
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
                let target = if flags & 0x20000 != 0 {
                    &mut triggers
                } else {
                    &mut faces
                };
                target.push(CollisionFace {
                    index,
                    vertices,
                    normal,
                    bounds,
                    flags,
                    face_flags: loaded_face_word(face.flags),
                });
            }
            items.push(CollisionItem {
                position: item.position_fixed,
                model: item.model_index as usize,
                faces,
                triggers,
                checksum: model.checksum,
                shatters: u16::from_le_bytes([model.header[0], model.header[1]]) & 2 != 0,
                panes: model
                    .faces
                    .iter()
                    .map(|face| {
                        let word = loaded_face_word(face.flags);
                        Pane {
                            breakable: face.raw.get(12..16).is_some_and(|w| {
                                u32::from_le_bytes(w.try_into().unwrap()) & BREAKABLE != 0
                            }),
                            textured: word & 0x40 == 0 && word & 1 != 0,
                            triangle: face.flags & 0x10 != 0,
                        }
                    })
                    .collect(),
            });
        }
        Ok(Self {
            zone: Arc::new(zone),
            hidden: vec![false; items.len()],
            items: items.into(),
            shattered: Vec::new(),
        })
    }

    /// Hides or shows the items whose origin is inside the box, or outside it on every axis.
    pub fn show_items(&mut self, [min, max]: [[i32; 3]; 2], inside: bool, show: bool) {
        for (item, hidden) in self.items.iter().zip(&mut self.hidden) {
            let p = item.position;
            let mut within = (0..3).map(|a| (min[a]..=max[a]).contains(&p[a]));
            let hit = if inside {
                within.all(|w| w)
            } else {
                within.all(|w| !w)
            };
            if hit {
                *hidden = !show;
            }
        }
    }

    /// Items a kill has shattered, in shatter order.
    pub fn shattered(&self) -> &[usize] {
        &self.shattered
    }

    /// Items the region scripts have hidden, in item order.
    pub fn hidden(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.hidden.len()).filter(|&i| self.hidden[i])
    }

    /// Breaks the glass of the item with this model checksum and returns its faces, or nothing
    /// when no item has it or its model cannot shatter. Faces already broken stay unbreakable.
    pub fn shatter(&mut self, checksum: u32) -> Option<Vec<Pane>> {
        let index = self.item_named(checksum)?;
        let item = &self.items[index];
        if !item.shatters {
            return None;
        }
        let broken = self.shattered.contains(&index);
        let panes = item
            .panes
            .iter()
            .map(|p| Pane {
                breakable: p.breakable && !broken,
                ..*p
            })
            .collect();
        if !broken {
            self.shattered.push(index);
        }
        Some(panes)
    }

    /// Shows or hides the first item with this model checksum.
    pub fn show_item(&mut self, checksum: u32, show: bool) {
        if let Some(index) = self.item_named(checksum) {
            self.hidden[index] = !show;
        }
    }

    /// Scripts name items by model checksum and the level loader keeps the scene's item order.
    fn item_named(&self, checksum: u32) -> Option<usize> {
        self.items.iter().position(|item| item.checksum == checksum)
    }

    /// Model checksums of the items whose trigger faces the segment crosses, in walk order.
    pub fn crossed_triggers(&self, start: [i32; 3], end: [i32; 3]) -> Result<Vec<u32>, String> {
        let mut fired = Vec::new();
        if start == end {
            return Ok(fired);
        }
        let mut stamped = vec![false; self.items.len()];
        for cell in self.zone.walk([start[0], start[2]], [end[0], end[2]]) {
            for &item_index in &self.zone.cell(cell).unwrap().items {
                if std::mem::replace(&mut stamped[item_index], true) || self.hidden[item_index] {
                    continue;
                }
                let item = &self.items[item_index];
                let (s, e) = (item.local(start), item.local(end));
                for face in &item.triggers {
                    if crossing(face, start, end, s, e)?.is_some() {
                        fired.push(item.checksum);
                        break;
                    }
                }
            }
        }
        Ok(fired)
    }

    /// Q12 endpoints, static skater filter ignoring lip faces, no trigger execution or moving objects.
    pub fn query(&self, start: [i32; 3], end: [i32; 3]) -> Result<Option<Hit>, String> {
        self.query_filtered(start, end, true)
    }

    /// Lip faces carry bit 22 and are skipped only while the native probe flag is raised.
    pub fn query_filtered(
        &self,
        start: [i32; 3],
        end: [i32; 3],
        ignore_lips: bool,
    ) -> Result<Option<Hit>, String> {
        if start == end {
            return Ok(None);
        }
        let delta: [i64; 3] = std::array::from_fn(|a| i64::from(end[a]) - i64::from(start[a]));
        if delta.iter().any(|&v| i32::try_from(v).is_err()) {
            return Err("collision query delta overflow".into());
        }
        // Native folds the unit deltas through shorts before measuring the segment.
        let units = delta.map(|v| i64::from((v >> 12) as i16));
        let length = if units[0] == 0 && units[2] == 0 {
            units[1].abs()
        } else {
            (units.iter().map(|&v| (v * v) as f64).sum::<f64>()).sqrt() as i64
        };
        if length == 0 {
            return Ok(None);
        }
        let query = Query {
            start,
            end,
            delta,
            length,
            ignore_lips,
        };
        let mut best = None;
        let mut stamped = vec![false; self.items.len()];
        for cell in self.zone.walk([start[0], start[2]], [end[0], end[2]]) {
            for &item_index in &self.zone.cell(cell).unwrap().items {
                if !std::mem::replace(&mut stamped[item_index], true) && !self.hidden[item_index] {
                    self.test_item(item_index, &query, &mut best)?;
                }
            }
        }
        Ok(best)
    }

    fn test_item(
        &self,
        item_index: usize,
        query: &Query,
        best: &mut Option<Hit>,
    ) -> Result<(), String> {
        let Query {
            start,
            end,
            delta,
            length,
            ignore_lips,
        } = *query;
        let item = &self.items[item_index];
        let (s, e) = (item.local(start), item.local(end));
        let broken = self.shattered.contains(&item_index);
        for face in &item.faces {
            if (ignore_lips && face.flags & 0x400000 != 0)
                || (broken && face.flags & BREAKABLE != 0)
            {
                continue;
            }
            let Some((d0, d1)) = crossing(face, start, end, s, e)? else {
                continue;
            };
            let denominator =
                i32::try_from(d0 - d1).map_err(|_| "collision plane difference overflow")?;
            let t = d0 * 0x4000 / i64::from(denominator);
            if best.is_some_and(|hit| i64::from(hit.t_q14) <= t) {
                continue;
            }
            let position_fixed =
                std::array::from_fn(|a| (i64::from(start[a]) + delta[a] * t / 0x4000) as i32);
            *best = Some(Hit {
                item_index,
                model_index: item.model,
                face_index: face.index,
                t_q14: t as i32,
                position_fixed,
                normal: face.normal,
                distance_units: (length * t / 0x4000) as i32,
                flags: face.flags,
                face_flags: face.face_flags,
            });
        }
        Ok(())
    }
}

impl CollisionItem {
    fn local(&self, p: [i32; 3]) -> [i64; 3] {
        std::array::from_fn(|a| i64::from(p[a] >> 12) - i64::from(self.position[a] >> 12))
    }
}

/// Plane distances of a segment entering the face from the front and passing inside every edge.
fn crossing(
    face: &CollisionFace,
    start: [i32; 3],
    end: [i32; 3],
    s: [i64; 3],
    e: [i64; 3],
) -> Result<Option<(i64, i64)>, String> {
    if (0..3).any(|a| {
        face.bounds[0][a] > i64::from(start[a].max(end[a]) & !0xfff) + 0x4000
            || face.bounds[1][a] < i64::from(start[a].min(end[a]) & !0xfff) - 0x4000
    }) {
        return Ok(None);
    }
    let plane = |p: [i64; 3]| -> Result<i64, String> {
        let distance: i64 = (0..3)
            .map(|a| i64::from(face.normal[a]) * (p[a] - i64::from(face.vertices[0][a])))
            .sum();
        i32::try_from(distance)
            .map(i64::from)
            .map_err(|_| "collision plane distance overflow".into())
    };
    let d0 = plane(s)?;
    let d1 = plane(e)?;
    if d0 < 0 || d1 > 0 || (d0 < 0x800 && d1 > -0x800) {
        return Ok(None);
    }
    let direction: [f64; 3] = std::array::from_fn(|a| (e[a] - s[a]) as f64);
    for edge in 0..face.vertices.len() {
        let a: [f64; 3] =
            std::array::from_fn(|i| (i64::from(face.vertices[edge][i]) - s[i]) as f64);
        let b: [f64; 3] = std::array::from_fn(|i| {
            (i64::from(face.vertices[(edge + 1) % face.vertices.len()][i]) - s[i]) as f64
        });
        let cross = [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ];
        if (0..3).map(|i| cross[i] * direction[i]).sum::<f64>() < 0.0 {
            return Ok(None);
        }
    }
    Ok(Some((d0, d1)))
}

struct Query {
    start: [i32; 3],
    end: [i32; 3],
    delta: [i64; 3],
    length: i64,
    ignore_lips: bool,
}

/// Native a*b/c truncates an x87 quotient, exact division only differs within one extended ulp.
fn muldiv(a: i64, b: i64, c: i64) -> i64 {
    (i128::from(a) * i128::from(b) / i128::from(c)) as i64
}
