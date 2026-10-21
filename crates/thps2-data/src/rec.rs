/// Replay block as the original saves it: a header, then the per-frame skater state bitstream.
#[derive(Debug)]
pub struct Recording {
    pub frames: u32,
    pub segments: [[u32; 2]; 5],
    pub skaters: i16,
    pub skater_ids: [u8; 2],
    pub level: i16,
    pub mode: i16,
    pub data: Vec<u8>,
    /// Title from the memory card style prefix of a saved replay.
    pub save_name: Option<String>,
}

const HEADER: usize = 0x36;
const CARD_PREFIX: usize = 0x200;

fn word(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}

fn half(data: &[u8], offset: usize) -> i16 {
    i16::from_le_bytes(data[offset..offset + 2].try_into().unwrap())
}

impl Recording {
    pub fn parse(data: &[u8]) -> Result<Self, String> {
        if data.len() < HEADER {
            return Err(format!(
                "recording truncated size={} header={HEADER}",
                data.len()
            ));
        }
        let skaters = half(data, 0x2c);
        if !(1..=2).contains(&skaters) {
            return Err(format!("recording skaters={skaters} outside 1..=2"));
        }
        let mut segments = [[0; 2]; 5];
        for (i, pair) in segments.iter_mut().enumerate() {
            *pair = [word(data, 4 + i * 8), word(data, 8 + i * 8)];
        }
        Ok(Self {
            frames: word(data, 0),
            segments,
            skaters,
            skater_ids: [data[0x2e], data[0x2f]],
            level: half(data, 0x32),
            mode: half(data, 0x34),
            data: data.to_vec(),
            save_name: None,
        })
    }

    /// Saved replays carry a 0x200 byte card prefix, the block follows at file offset 0x200.
    pub fn parse_save(file: &[u8]) -> Result<Self, String> {
        if file.len() < CARD_PREFIX + HEADER {
            return Err(format!("saved replay truncated size={}", file.len()));
        }
        if &file[..2] != b"SC" {
            return Err(format!("saved replay card magic={:02x?}", &file[..4]));
        }
        let title = &file[4..0x44];
        let end = title.iter().position(|&b| b == 0).unwrap_or(title.len());
        let mut r = Self::parse(&file[CARD_PREFIX..])?;
        r.save_name = Some(String::from_utf8_lossy(&title[..end]).into_owned());
        Ok(r)
    }
}

/// Byte offset of the first stream word inside a replay block.
pub const STREAM: usize = 0x4800;
const POS_BITS: [u32; 7] = [3, 4, 5, 7, 8, 16, 20];
const ROT_BITS: [u32; 7] = [2, 4, 5, 6, 7, 9, 12];
const FLIP_BITS: [u32; 5] = [0, 6, 7, 8, 12];

/// Skater values the decoder carries in from before playback.
#[derive(Clone, Copy, Debug)]
pub struct Start {
    pub position: [i32; 3],
    pub state: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sound {
    pub kind: bool,
    pub id: u16,
}

/// Skater state the original writes for one playback frame, positions in world units.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReplayFrame {
    pub cars: [([i32; 3], [i16; 3]); 8],
    pub position: [i32; 3],
    pub old_position: [i32; 3],
    pub velocity: [i32; 3],
    /// Euler angles x y z in 4096 units per turn, the matrix built from them is not reproduced
    pub euler: [u16; 3],
    pub state: i32,
    pub anim: u16,
    pub anim_frame: i16,
    pub flag: bool,
    pub flip_on: bool,
    pub flip_angle: i32,
    pub vb_delta: u32,
    pub sound: Option<Sound>,
    pub splat: Option<u8>,
    pub trick_object: Option<u16>,
}

struct Bits<'a> {
    data: &'a [u8],
    next: usize,
    cur: u32,
    left: u32,
}

impl Bits<'_> {
    fn load(&mut self) -> Result<u32, String> {
        let word = self
            .data
            .get(self.next..self.next + 4)
            .ok_or_else(|| format!("replay stream ends at offset={}", self.next))?;
        self.next += 4;
        Ok(u32::from_le_bytes(word.try_into().unwrap()))
    }

    fn bits(&mut self, n: u32) -> Result<u32, String> {
        if self.left == 0 {
            self.cur = self.load()?;
            self.left = 32;
        }
        let mask = |k: u32| if k == 32 { u32::MAX } else { (1 << k) - 1 };
        if n <= self.left {
            let v = self.cur & mask(n);
            self.cur = self.cur.checked_shr(n).unwrap_or(0);
            self.left -= n;
            return Ok(v);
        }
        let (k, r) = (self.left, n - self.left);
        let word = self.load()?;
        let v = (self.cur & mask(k)) | ((word & mask(r)) << k);
        self.cur = word >> r;
        self.left = 32 - r;
        Ok(v)
    }

    fn bit(&mut self) -> Result<bool, String> {
        Ok(self.bits(1)? == 1)
    }

    fn signed(&mut self, n: u32) -> Result<i32, String> {
        let v = self.bits(n)?;
        Ok(if v & (1 << (n - 1)) != 0 {
            (v | !((1u32 << n) - 1)) as i32
        } else {
            v as i32
        })
    }

    fn unary(&mut self) -> Result<usize, String> {
        let mut i = 0;
        while i < 6 && self.bit()? {
            i += 1;
        }
        Ok(i)
    }
}

/// Decodes a full playback the way the PC reader does, from block offset 0x4800.
pub fn decode_stream(block: &[u8], frames: u32, start: Start) -> Result<Vec<ReplayFrame>, String> {
    let mut b = Bits {
        data: block,
        next: STREAM,
        cur: 0,
        left: 0,
    };
    let (mut prev, mut prev2) = ([0i32; 3], [0i32; 3]);
    let mut euler = [0u16; 3];
    let (mut flip, mut flip_res) = (0i32, 0i32);
    let mut last = ReplayFrame {
        position: start.position,
        state: start.state,
        ..ReplayFrame::default()
    };
    let mut out = Vec::with_capacity(frames as usize);
    for _ in 0..frames {
        let mut f = ReplayFrame {
            state: last.state,
            anim: last.anim,
            flip_angle: last.flip_angle,
            ..ReplayFrame::default()
        };
        for car in &mut f.cars {
            for axis in &mut car.0 {
                *axis = (b.bits(20)? << 12) as i32;
            }
            for axis in &mut car.1 {
                *axis = b.bits(16)? as u16 as i16;
            }
        }
        let width = POS_BITS[b.unary()?];
        let mut p = [0i32; 3];
        for a in 0..3 {
            p[a] = b
                .signed(width)?
                .wrapping_add(prev[a].wrapping_mul(2))
                .wrapping_sub(prev2[a]);
        }
        (prev2, prev) = (prev, p);
        f.old_position = last.position;
        f.position = p.map(|v| v.wrapping_shl(12));
        for a in 0..3 {
            f.velocity[a] = f.position[a].wrapping_sub(f.old_position[a]);
        }
        let width = ROT_BITS[b.unary()?];
        for angle in &mut euler {
            *angle = (*angle).wrapping_add((b.signed(width)? & 0xfff) as u16) & 0xfff;
        }
        f.euler = euler;
        if b.bit()? {
            f.state = b.bits(4)? as i32;
        }
        if b.bit()? {
            f.anim = b.bits(8)? as u16;
        }
        f.anim_frame = if b.bit()? {
            b.bits(8)? as i16
        } else {
            last.anim_frame.wrapping_add(b.bits(2)? as i16)
        };
        f.flag = b.bit()?;
        if b.bit()? {
            f.flip_on = true;
            let index = b.unary()?;
            let width = *FLIP_BITS
                .get(index)
                .ok_or_else(|| format!("replay flip width index={index} undefined"))?;
            if width != 0 {
                let r = b.signed(width)?;
                f.flip_angle = flip_res.wrapping_add(r).wrapping_add(flip);
                (flip, flip_res) = (f.flip_angle, r);
            }
        } else {
            (flip, flip_res, f.flip_angle) = (0, 0, 0);
        }
        f.vb_delta = b.bits(4)?;
        if b.bit()? {
            let kind = b.bit()?;
            f.sound = Some(Sound {
                kind,
                id: b.bits(9)? as u16,
            });
        }
        if b.bit()? {
            if b.bit()? {
                f.splat = Some(b.bits(2)? as u8);
            }
            if b.bit()? {
                f.trick_object = Some(b.bits(9)? as u16);
            }
        }
        out.push(f);
        last = f;
    }
    Ok(out)
}
