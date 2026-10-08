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

/// Object animation cursor: frame and sixteen bit fraction advance by speed times the frame scale.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnimState {
    pub id: u16,
    pub frame: i16,
    pub fraction: u16,
    pub speed: i32,
    pub direction: i8,
    pub end: i8,
    pub pingpong: i8,
    pub count: u8,
    pub finished: bool,
    pub mode: u8,
    pub start: i16,
}

impl Default for AnimState {
    fn default() -> Self {
        Self {
            id: 0,
            frame: 0,
            fraction: 0,
            speed: 0x10000,
            direction: 0,
            end: 0,
            pingpong: 0,
            count: 1,
            finished: true,
            mode: 0,
            start: 0,
        }
    }
}

const FALLBACK_ANIM: u16 = 0x2e;

impl AnimState {
    /// Runs an animation from start to end at one design frame per frame, minus one meaning the last frame.
    pub fn run(
        &mut self,
        table: &AnimationTable,
        id: u16,
        start: i32,
        end: i32,
        pingpong: i8,
    ) -> Result<(), String> {
        if self.id == 0x82 || id == 0x82 {
            return Err("stance change animation unsupported".into());
        }
        self.speed = 0x10000;
        let id = if usize::from(id) >= table.entries.len() {
            FALLBACK_ANIM
        } else {
            id
        };
        let count = table
            .frame_count(usize::from(id))
            .ok_or_else(|| format!("animation id={id} missing from the bank table"))?;
        let clamp = |f: i32| if f == -1 { count - 1 } else { f }.clamp(0, count - 1);
        let (start, end) = (clamp(start), clamp(end));
        self.id = id;
        self.count = count as u8;
        self.mode = 0;
        self.end = end as i8;
        self.direction = (end - start).signum() as i8;
        self.pingpong = pingpong;
        self.start = start as i16;
        self.frame = start as i16;
        self.fraction = 0;
        self.finished = start == end;
        Ok(())
    }

    /// Plays the animation back from the current frame to its start frame.
    pub fn reverse(&mut self) {
        self.finished = false;
        self.direction = -1;
        (self.end, self.start) = (self.start as i8, i16::from(self.end));
    }

    fn at_end(&self) -> bool {
        (self.direction == 1 && self.frame >= i16::from(self.end))
            || (self.direction == -1 && self.frame <= i16::from(self.end))
    }

    /// Advances by speed times the Q8 frame scale, flipping at a pingpong end or finishing.
    pub fn tick(&mut self, scale: i32) -> Result<(), String> {
        if self.mode != 0 {
            return Err(format!("animation mode={} unsupported", self.mode));
        }
        if self.at_end() {
            if self.pingpong < 1 {
                self.finished = true;
            } else {
                (self.end, self.pingpong) = (self.pingpong, self.end);
                self.direction = -self.direction;
            }
        }
        let step = (self.speed * scale) >> 8;
        let mut cursor = (i32::from(self.frame) << 16) | i32::from(self.fraction);
        cursor = match self.direction {
            1 => cursor.wrapping_add(step),
            -1 => cursor.wrapping_sub(step),
            _ => cursor,
        };
        self.fraction = cursor as u16;
        self.frame = (cursor >> 16) as i16;
        if self.at_end() {
            self.frame = i16::from(self.end);
        }
        Ok(())
    }
}
