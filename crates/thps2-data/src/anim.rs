use crate::psx::Scene;

pub const ANIMATION_CHUNK: u32 = 0x2c;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnimationEntry {
    pub frame_count: u8,
    pub data_offset: u32,
}

/// Bank animation table from the PSX animation chunk, frame counts first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnimationTable {
    pub data_base: u32,
    pub chunk_len: usize,
    pub entries: Vec<AnimationEntry>,
}

impl AnimationTable {
    pub fn parse(data: &[u8]) -> Result<Self, String> {
        let word = |offset: usize| -> Result<u32, String> {
            let bytes = data
                .get(offset..offset + 4)
                .ok_or_else(|| format!("animation table offset={offset} truncated word"))?;
            Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
        };
        let count = word(0)? as usize;
        let data_base = word(4)?;
        if data.len() < 8 + count * 8 {
            return Err(format!(
                "animation table count={count} needs {} bytes, chunk has {}",
                8 + count * 8,
                data.len()
            ));
        }
        let mut entries = Vec::with_capacity(count);
        for id in 0..count {
            let entry = &data[8 + id * 8..16 + id * 8];
            if entry[1..4] != [0, 0, 0] {
                return Err(format!(
                    "animation table id={id} reserved bytes={:02x}{:02x}{:02x} are not zero",
                    entry[1], entry[2], entry[3]
                ));
            }
            entries.push(AnimationEntry {
                frame_count: entry[0],
                data_offset: u32::from_le_bytes(entry[4..8].try_into().unwrap()),
            });
        }
        Ok(Self {
            data_base,
            chunk_len: data.len(),
            entries,
        })
    }

    pub fn from_scene(scene: &Scene) -> Result<Self, String> {
        let mut chunks = scene.chunks.iter().filter(|c| c.kind == ANIMATION_CHUNK);
        let chunk = chunks.next().ok_or("scene has no animation chunk 0x2c")?;
        if chunks.next().is_some() {
            return Err("scene has multiple animation chunks".into());
        }
        Self::parse(&chunk.payload)
    }

    pub fn frame_count(&self, id: usize) -> Option<i32> {
        self.entries.get(id).map(|e| i32::from(e.frame_count))
    }

    /// Default end frame used when an animation runs to its last frame.
    pub fn last_frame(&self, id: usize) -> Option<i32> {
        self.frame_count(id).map(|count| (count - 1).max(0))
    }

    /// Entries whose data offset lies outside the chunk.
    pub fn unreachable_entries(&self) -> Vec<usize> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.data_offset as usize >= self.chunk_len)
            .map(|(id, _)| id)
            .collect()
    }
}
