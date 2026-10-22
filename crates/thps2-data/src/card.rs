/// Saves keep the PlayStation memory card header in front of their data.
pub const CARD_HEADER: usize = 0x200;

const TITLE_SPACE: usize = 0x40;
const PALETTE: u32 = 0x519244;
const FRAMES: [u32; 2] = [0x5191c4, 0x519264];

/// The career save icon: a 16 colour palette and two 16x16 animation frames, read from the exe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardIcons {
    palette: [u8; 0x20],
    frames: [[u8; 0x80]; 2],
}

/// File offset of a virtual address inside a section's stored bytes.
fn file_offset(exe: &[u8], va: u32, len: usize) -> Result<usize, String> {
    let word = |at: usize| -> Result<u32, String> {
        exe.get(at..at + 4)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
            .ok_or_else(|| format!("exe truncated at={at:#x}"))
    };
    if exe.get(..2) != Some(b"MZ") {
        return Err("exe missing MZ header".into());
    }
    let pe = word(0x3c)? as usize;
    let sections = word(pe + 4)? >> 16;
    let table = pe + 24 + (word(pe + 20)? & 0xffff) as usize;
    let base = word(pe + 52)?;
    for s in (0..sections as usize).map(|i| table + 40 * i) {
        let (start, size, raw) = (base + word(s + 12)?, word(s + 16)?, word(s + 20)?);
        if (start..start + size).contains(&va) {
            let at = (raw + va - start) as usize;
            return match exe.get(at..at + len) {
                Some(_) => Ok(at),
                None => Err(format!("exe truncated va={va:#x}")),
            };
        }
    }
    Err(format!("exe has no section holding va={va:#x}"))
}

impl CardIcons {
    pub fn from_exe(exe: &[u8]) -> Result<Self, String> {
        let take = |va: u32, len: usize| file_offset(exe, va, len).map(|at| &exe[at..at + len]);
        let mut icons = Self {
            palette: take(PALETTE, 0x20)?.try_into().unwrap(),
            frames: [[0; 0x80]; 2],
        };
        for (frame, va) in icons.frames.iter_mut().zip(FRAMES) {
            frame.copy_from_slice(take(va, 0x80)?);
        }
        Ok(icons)
    }

    /// The header the original writes: magic, frame count, block count, title, palette and
    /// three icon slots, the last repeating the first frame.
    pub fn header(&self, title: &str, blocks: u8) -> Result<[u8; CARD_HEADER], String> {
        if title.len() >= TITLE_SPACE || !title.is_ascii() {
            return Err(format!("card title unsupported title={title:?}"));
        }
        let mut h = [0; CARD_HEADER];
        h[..4].copy_from_slice(&[b'S', b'C', 0x10 + 2, blocks]);
        h[4..4 + title.len()].copy_from_slice(title.as_bytes());
        h[0x60..0x80].copy_from_slice(&self.palette);
        for (slot, frame) in [0, 1, 0].into_iter().enumerate() {
            h[0x80 * (slot + 1)..0x80 * (slot + 2)].copy_from_slice(&self.frames[frame]);
        }
        Ok(h)
    }
}
