use crate::{bitmap::Bitmap, raster::Texture};

/// A 2D image as the game uploads it, padded, keyed and stretched to a power of two surface.
pub struct Sprite {
    pub width: u16,
    pub height: u16,
    /// The padded size texture coordinates are scaled by.
    pub logical: [usize; 2],
    pub texture: Texture,
}

fn argb1555([r, g, b]: [u8; 3]) -> u16 {
    if r >= 0xf8 && g == 0 && b >= 0xf8 {
        return 0;
    }
    0x8000 | u16::from(r >> 3) << 10 | u16::from(g >> 3) << 5 | u16::from(b >> 3)
}

impl Sprite {
    pub fn from_bmp(bytes: &[u8]) -> Result<Self, String> {
        let image = Bitmap::parse(bytes)?;
        let pad = *image
            .palette
            .first()
            .ok_or("[sprite] bitmap has no palette")?;
        let (w, h) = (usize::from(image.width), usize::from(image.height));
        let padded = w.next_multiple_of(4);
        let pixel = |x: usize, y: usize| -> u16 {
            if x >= w {
                return argb1555(pad);
            }
            let at = (y * w + x) * 3;
            argb1555([image.rgb[at], image.rgb[at + 1], image.rgb[at + 2]])
        };
        let (width, height) = (padded.next_power_of_two(), h.next_power_of_two());
        let texels = (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .map(|(x, y)| pixel(x * padded / width, y * h / height))
            .collect();
        Ok(Self {
            width: image.width,
            height: image.height,
            logical: [padded, h],
            texture: Texture {
                width,
                height,
                texels,
            },
        })
    }
}
