/// Map values from 0xf8 up are colour codes and the blank, so glyph numbers stay below.
const MAX_GLYPHS: u32 = 0xf8;
/// The blank glyph that unmapped characters draw is glyph 0's box filled with this byte.
const BLANK_BYTE: u8 = 0x12;

#[derive(Debug)]
pub struct Glyph {
    pub width: u8,
    pub height: u8,
    /// Rows above the baseline.
    pub base: u8,
    /// Palette indices, row by row, width times height.
    pub pixels: Vec<u8>,
}

/// A 4 bpp bitmap font: one glyph per character slot sharing a 16 entry PS1 palette, plus the
/// blank glyph the loader appends.
#[derive(Debug)]
pub struct Font {
    pub glyphs: Vec<Glyph>,
    pub clut: [u16; 16],
}

impl Font {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let take = |at: usize, len: usize| {
            bytes
                .get(at..at + len)
                .ok_or_else(|| format!("[fnt] truncated at={at} need={len} size={}", bytes.len()))
        };
        let word = |at: usize| take(at, 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
        let count = word(0)?;
        if count == 0 || count >= MAX_GLYPHS {
            return Err(format!("[fnt] glyphs={count} outside 1..{MAX_GLYPHS}"));
        }
        let clut_at = 4 + 16 * count as usize;
        let clut_bytes = take(clut_at, 32)?;
        let clut =
            std::array::from_fn(|i| u16::from_le_bytes([clut_bytes[2 * i], clut_bytes[2 * i + 1]]));
        let mut at = clut_at + 32;
        let mut glyphs = Vec::with_capacity(count as usize + 1);
        for i in 0..count as usize {
            let header = take(4 + 16 * i, 16)?;
            let [stride, height, base, width] = std::array::from_fn(|k| {
                u32::from_le_bytes(header[4 * k..4 * k + 4].try_into().unwrap())
            });
            if width == 0 || width > 255 || height > 255 || base > 255 || stride * 4 < width {
                return Err(format!(
                    "[fnt] glyph={i} stride={stride} width={width} height={height} base={base}"
                ));
            }
            let row = stride as usize * 2;
            glyphs.push(Glyph {
                width: width as u8,
                height: height as u8,
                base: base as u8,
                pixels: unpack(take(at, row * height as usize)?, row, width as usize),
            });
            at += row * height as usize + if stride * height % 2 == 1 { 2 } else { 0 };
        }
        take(at, 0)?;
        if at != bytes.len() {
            return Err(format!(
                "[fnt] size={} expected={at} trailing={}",
                bytes.len(),
                bytes.len() - at
            ));
        }
        let first = &glyphs[0];
        let row = first.width.div_ceil(4) as usize * 2;
        let blank = vec![BLANK_BYTE; row * first.height as usize];
        glyphs.push(Glyph {
            pixels: unpack(&blank, row, first.width as usize),
            ..*first
        });
        Ok(Self { glyphs, clut })
    }
}

/// Low nibble first, rows cut to the glyph width.
fn unpack(rows: &[u8], row: usize, width: usize) -> Vec<u8> {
    rows.chunks_exact(row)
        .flat_map(|r| r.iter().flat_map(|b| [b & 0xf, b >> 4]).take(width))
        .collect()
}
