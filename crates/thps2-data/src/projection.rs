use crate::draw::{PreparedVertex, ProjectedVertex};

#[derive(Clone, Copy, Debug)]
pub struct ClippedVertex {
    pub projected: ProjectedVertex,
    /// Native left/right/top/bottom/near/far bits 0–5 and optional plane bit 8.
    pub clip: u16,
}

pub fn face_outside_clip_volume(vertices: &[ClippedVertex]) -> Result<bool, String> {
    if !(3..=4).contains(&vertices.len()) {
        return Err("face clipping requires three or four corners".into());
    }
    let common = vertices.iter().fold(u16::MAX, |flags, v| flags & v.clip);
    let any = vertices.iter().fold(0, |flags, v| flags | v.clip);
    Ok(common & 0x10 != 0 || (common & 0x3f != 0 && any & 0x10 == 0))
}

#[derive(Clone, Copy, Debug)]
pub struct StaticView {
    position: [i32; 3],
    matrix: [[f32; 3]; 3],
}

#[derive(Clone, Copy, Debug)]
pub struct ModelView {
    matrix: [[f32; 3]; 3],
    translation: [f32; 3],
}

fn multiply(matrix: [[f32; 3]; 3], v: [f32; 3]) -> [f32; 3] {
    matrix.map(|row| row[0] * v[0] + row[1] * v[1] + row[2] * v[2])
}

impl StaticView {
    /// Camera position uses integer source units, matrix rows use signed Q12.
    pub fn new(position: [i32; 3], matrix_q12: [[i16; 3]; 3]) -> Self {
        Self {
            position,
            matrix: matrix_q12.map(|row| row.map(|v| v as f32 / 4096.0)),
        }
    }

    pub fn item_transform(
        &self,
        position_fixed: [i32; 3],
        rotation: [i16; 3],
    ) -> Result<ModelView, String> {
        if rotation != [0; 3] {
            return Err(format!(
                "static view unsupported item rotation={rotation:?}"
            ));
        }
        let mut relative = [0.0; 3];
        for axis in 0..3 {
            relative[axis] = (position_fixed[axis] >> 12)
                .checked_sub(self.position[axis])
                .ok_or_else(|| format!("static view translation overflow axis={axis}"))?
                as f32;
        }
        Ok(ModelView {
            matrix: self.matrix,
            translation: multiply(self.matrix, relative),
        })
    }
}

impl ModelView {
    pub fn transform(&self, position: [i16; 3]) -> [f32; 3] {
        let rotated = multiply(self.matrix, position.map(f32::from));
        std::array::from_fn(|axis| rotated[axis] + self.translation[axis])
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Projection {
    viewport: [f32; 4],
    center: [f32; 2],
    focal: f32,
    depth: [f32; 2],
}

impl Projection {
    /// Bounds are left/top/right/bottom pixels and near/far camera-space depths.
    /// Supply the effective far depth after the native viewport's 1.5 multiplier.
    pub fn new(
        viewport: [f32; 4],
        center: [f32; 2],
        focal: f32,
        depth: [f32; 2],
    ) -> Result<Self, String> {
        if viewport
            .iter()
            .chain(&center)
            .chain(&depth)
            .chain([&focal])
            .any(|v| !v.is_finite())
            || viewport[0] >= viewport[2]
            || viewport[1] >= viewport[3]
            || focal <= 0.0
            || depth[0] <= 0.0
            || depth[0] >= depth[1]
        {
            return Err(
                "projection requires finite, ordered bounds and positive focal/near depth".into(),
            );
        }
        Ok(Self {
            viewport,
            center,
            focal,
            depth,
        })
    }

    pub fn classify(&self, projected: ProjectedVertex) -> Result<ClippedVertex, String> {
        if projected
            .pixel
            .iter()
            .chain([&projected.depth, &projected.scale])
            .any(|v| !v.is_finite())
        {
            return Err("projection contains non-finite values".into());
        }
        let [x, y] = projected.pixel;
        let clip = u16::from(x < self.viewport[0])
            | (u16::from(x >= self.viewport[2]) << 1)
            | (u16::from(y < self.viewport[1]) << 2)
            | (u16::from(y >= self.viewport[3]) << 3)
            | (u16::from(projected.depth < self.depth[0]) << 4)
            | (u16::from(projected.depth >= self.depth[1]) << 5);
        Ok(ClippedVertex { projected, clip })
    }

    /// Normal projection mode only, with positive camera Z forward and Y downward.
    pub fn project(&self, camera: [f32; 3]) -> Result<ClippedVertex, String> {
        if camera.iter().any(|v| !v.is_finite()) || camera[2] == 0.0 {
            return Err("projection requires finite camera coordinates and nonzero depth".into());
        }
        let scale = self.focal / camera[2];
        self.classify(ProjectedVertex {
            pixel: [
                scale * camera[0] + self.center[0],
                scale * camera[1] + self.center[1],
            ],
            depth: camera[2],
            scale,
        })
    }

    pub fn resolve_offset(
        &self,
        vertex: &PreparedVertex,
        previous: &[ClippedVertex],
    ) -> Result<ClippedVertex, String> {
        let PreparedVertex::ScreenOffset { anchors, distance } = vertex else {
            return Err("offset resolution requires a screen-offset vertex".into());
        };
        let mut pair = Vec::with_capacity(2);
        for &anchor in anchors {
            pair.push(
                previous
                    .get(anchor)
                    .ok_or_else(|| {
                        format!("offset anchor={anchor} projected_count={}", previous.len())
                    })?
                    .projected,
            );
        }
        let projected = PreparedVertex::ScreenOffset {
            anchors: [0, 1],
            distance: *distance,
        }
        .resolve_offset(&pair)?;
        let mut output = self.classify(projected)?;
        output.clip |= previous[anchors[0]].clip & 0x100;
        Ok(output)
    }

    pub fn project_model(
        &self,
        vertices: &[PreparedVertex],
        transform: &ModelView,
    ) -> Result<Vec<ClippedVertex>, String> {
        let mut output = Vec::with_capacity(vertices.len());
        for (index, vertex) in vertices.iter().enumerate() {
            let projected = match vertex {
                PreparedVertex::Position(position) => self.project(transform.transform(*position)),
                PreparedVertex::ScreenOffset { .. } => self.resolve_offset(vertex, &output),
            }
            .map_err(|error| format!("vertex={index} {error}"))?;
            output.push(projected);
        }
        Ok(output)
    }
}
