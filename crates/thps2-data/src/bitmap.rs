#[derive(Debug)]
pub struct Bitmap {
    pub width: u16,
    pub height: u16,
    // Top-to-bottom RGB bytes, before native surface quantization.
    pub rgb: Vec<u8>,
}

impl Bitmap {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 54 {
            return Err(format!("[bmp] size={} expected_header=54", bytes.len()));
        }
        let word = |offset| u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        let short = |offset| u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap());
        let width = word(18) as i32;
        let height = word(22) as i32;
        if &bytes[..2] != b"BM" || word(14) != 40 || word(10) != 54 {
            return Err(format!(
                "[bmp] unsupported header magic={:?} dib={} pixel_offset={}",
                &bytes[..2],
                word(14),
                word(10)
            ));
        }
        if short(26) != 1 || short(28) != 24 || word(30) != 0 {
            return Err(format!(
                "[bmp] unsupported planes={} depth={} compression={}",
                short(26),
                short(28),
                word(30)
            ));
        }
        if !(1..=32767).contains(&width) || !(1..=32767).contains(&height) {
            return Err(format!("[bmp] unsupported dimensions={width}x{height}"));
        }
        let stride = (width as usize * 3 + 3) & !3;
        let end = 54 + stride * height as usize;
        if end > bytes.len() || (word(2) as usize) < end || word(2) as usize > bytes.len() {
            return Err(format!(
                "[bmp] truncated pixels end={end} declared_size={} size={}",
                word(2),
                bytes.len()
            ));
        }
        let mut rgb = Vec::with_capacity(width as usize * height as usize * 3);
        for row in bytes[54..end].chunks_exact(stride).rev() {
            for bgr in row[..width as usize * 3].as_chunks::<3>().0 {
                rgb.extend_from_slice(&[bgr[2], bgr[1], bgr[0]]);
            }
        }
        Ok(Self {
            width: width as u16,
            height: height as u16,
            rgb,
        })
    }
}
