use crate::{bitmap::Bitmap, pkr::Archive, psx::Scene};

#[derive(Debug)]
pub enum TextureSource {
    Bitmap { path: String, image: Bitmap },
    PaletteWords(Vec<u16>),
}

#[derive(Debug)]
pub struct Texture {
    pub checksum: u32,
    pub logical_size: [u16; 2],
    pub flags: u32,
    pub source: TextureSource,
}

pub fn resolve_textures(
    archive: &Archive,
    checksums: &[u32],
    bank: &Scene,
    level: &str,
) -> Result<Vec<Texture>, String> {
    checksums
        .iter()
        .map(|&checksum| {
            let fail =
                |reason: String| format!("[texture] checksum={checksum:08X} reason={reason}");
            let mut matches = bank.embedded_textures.iter().filter(|texture| {
                bank.texture_checksums.get(texture.texture_index as usize) == Some(&checksum)
            });
            let embedded = matches
                .next()
                .ok_or_else(|| fail("missing embedded texture".into()))?;
            if matches.next().is_some() {
                return Err(fail("ambiguous embedded texture".into()));
            }
            let mut source = None;
            for path in [
                format!("newtex/{level}/{checksum:08X}.BMP"),
                format!("newtex/{checksum:08X}.BMP"),
            ] {
                if let Some(entry) = archive
                    .entries()
                    .iter()
                    .find(|e| e.path().eq_ignore_ascii_case(&path))
                {
                    let bytes = archive
                        .read(entry.path())
                        .map_err(|e| fail(e.to_string()))?;
                    let image =
                        Bitmap::parse(bytes).map_err(|e| fail(format!("entry={path} {e}")))?;
                    source = Some(TextureSource::Bitmap { path, image });
                    break;
                }
            }
            let source = match source {
                Some(source) => source,
                None => {
                    let palettes = match embedded.color_count {
                        16 => &bank.palettes16,
                        256 => &bank.palettes256,
                        count => return Err(fail(format!("unsupported color_count={count}"))),
                    };
                    let mut matches = palettes
                        .iter()
                        .filter(|p| p.checksum == embedded.palette_checksum);
                    let palette = matches.next().ok_or_else(|| {
                        fail(format!("missing palette={:08X}", embedded.palette_checksum))
                    })?;
                    if matches.next().is_some() {
                        return Err(fail(format!(
                            "ambiguous palette={:08X}",
                            embedded.palette_checksum
                        )));
                    }
                    let words = embedded
                        .indices
                        .iter()
                        .map(|&index| {
                            palette.colors.get(index as usize).copied().ok_or_else(|| {
                                fail(format!(
                                    "palette index={index} colors={}",
                                    palette.colors.len()
                                ))
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    TextureSource::PaletteWords(words)
                }
            };
            Ok(Texture {
                checksum,
                logical_size: [embedded.width, embedded.height],
                flags: embedded.flags,
                source,
            })
        })
        .collect()
}

#[derive(Debug)]
pub struct Surface1555 {
    pub width: u16,
    pub height: u16,
    pub pixels: Vec<u16>,
}

impl Texture {
    // Normal color mode in the preferred native ARGB1555 surface format.
    pub fn surface_1555(&self) -> Surface1555 {
        match &self.source {
            TextureSource::Bitmap { image, .. } => Surface1555 {
                width: image.width,
                height: image.height,
                pixels: image
                    .rgb
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .map(|&[r, g, b]| {
                        if r >= 248 && g == 0 && b >= 248 {
                            0
                        } else {
                            0x8000
                                | ((r as u16 >> 3) << 10)
                                | ((g as u16 >> 3) << 5)
                                | (b as u16 >> 3)
                        }
                    })
                    .collect(),
            },
            TextureSource::PaletteWords(words) => Surface1555 {
                width: self.logical_size[0],
                height: self.logical_size[1],
                pixels: words
                    .iter()
                    .map(|&word| {
                        if word & 0x7fff == 0x7c1f {
                            0
                        } else {
                            0x8000 | ((word & 0x1f) << 10) | (word & 0x3e0) | ((word >> 10) & 0x1f)
                        }
                    })
                    .collect(),
            },
        }
    }
}

/// Level textures the loader swaps for sponsor logos, in the order it fills them.
pub const LOGO_PLACEHOLDERS: [u32; 12] = [
    0x608d2937, 0x1151121a, 0xe80d5664, 0x80ae04a6, 0x5f5b1ae8, 0x8f9c07c3, 0x72f76064, 0xf207a90c,
    0xf210cc10, 0x177367bf, 0x477b70c5, 0x2cbc3cd0,
];
pub const TONY_HAWK_LOGOS: [&str; 4] = ["DAAdio", "D_BIRD", "D_FURY", "DaQUIC"];

/// Puts the skater's sponsor logos on the level's logo placeholders, the next sponsor for each,
/// after skipping start places like the loader's random pick. Width picks the file size 01 to 04.
pub fn apply_logos(
    textures: &mut [Texture],
    archive: &Archive,
    sponsors: &[&str],
    start: usize,
) -> Result<(), String> {
    let mut cursor = start % sponsors.len();
    for checksum in LOGO_PLACEHOLDERS {
        let Some(texture) = textures.iter_mut().find(|t| t.checksum == checksum) else {
            continue;
        };
        let size = match texture.logical_size[0] {
            32 => 1,
            64 => 2,
            96 => 3,
            128 => 4,
            w => {
                return Err(format!(
                    "[logo] checksum={checksum:08X} width={w} has no logo size"
                ));
            }
        };
        cursor = (cursor + 1) % sponsors.len();
        let path = format!("data/logos/{}0{size}.BMP", sponsors[cursor]);
        let entry = archive
            .entries()
            .iter()
            .find(|e| e.path().eq_ignore_ascii_case(&path))
            .ok_or_else(|| format!("[logo] checksum={checksum:08X} missing={path}"))?;
        let image = Bitmap::parse(archive.read(entry.path()).map_err(|e| e.to_string())?)
            .map_err(|e| format!("[logo] entry={path} {e}"))?;
        texture.logical_size = [image.width, image.height];
        texture.source = TextureSource::Bitmap { path, image };
    }
    Ok(())
}
