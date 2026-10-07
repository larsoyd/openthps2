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
