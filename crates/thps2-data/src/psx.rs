#[derive(Debug)]
pub struct Item {
    pub flags: [u16; 2],
    pub position_fixed: [i32; 3],
    pub rotation: [i16; 3],
    pub model_index: u16,
    pub extra: [u8; 12],
}

#[derive(Debug)]
pub struct Vertex {
    pub words: [i16; 3],
    pub flags: u16,
}

#[derive(Debug)]
pub struct Face {
    pub flags: u32,
    pub indices: Vec<u8>,
    pub texture_index: Option<u32>,
    pub raw: Vec<u8>,
}

#[derive(Debug)]
pub struct Model {
    pub header: [u8; 28],
    pub vertices: Vec<Vertex>,
    pub normals_raw: Vec<[u8; 8]>,
    pub faces: Vec<Face>,
    pub checksum: u32,
}

#[derive(Debug)]
pub struct Chunk {
    pub kind: u32,
    pub payload: Vec<u8>,
}

#[derive(Debug)]
pub struct EmbeddedTexture {
    pub flags: u32,
    pub color_count: u32,
    pub palette_checksum: u32,
    pub texture_index: u32,
    pub width: u16,
    pub height: u16,
    pub indices: Vec<u8>,
}

#[derive(Debug)]
pub struct Palette {
    pub checksum: u32,
    pub colors: Vec<u16>,
}

#[derive(Debug)]
pub struct Scene {
    pub header_word: u32,
    pub items: Vec<Item>,
    pub models: Vec<Model>,
    pub chunks: Vec<Chunk>,
    pub texture_checksums: Vec<u32>,
    pub palettes16: Vec<Palette>,
    pub palettes256: Vec<Palette>,
    pub embedded_textures: Vec<EmbeddedTexture>,
}

impl Scene {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let mut reader = Reader::new(bytes, 0);
        let header_word = reader.u32()?;
        let chunk_start = reader.u32()? as usize;
        let item_count = reader.u32()? as usize;
        let mut items = Vec::new();
        for record in reader.table(item_count, 36)?.as_chunks::<36>().0 {
            let mut r = Reader::new(record, 0);
            items.push(Item {
                flags: [r.u16()?, r.u16()?],
                position_fixed: [r.u32()? as i32, r.u32()? as i32, r.u32()? as i32],
                rotation: [r.u16()? as i16, r.u16()? as i16, r.u16()? as i16],
                model_index: r.u16()?,
                extra: r.take(12)?.try_into().unwrap(),
            });
        }
        let model_count = reader.u32()? as usize;
        let model_offsets = reader.words(model_count)?;
        let models_start = reader.position;
        if chunk_start < models_start || chunk_start > bytes.len() {
            return Err(failure(
                4,
                format!("invalid chunk offset={chunk_start} model_table_end={models_start}"),
            ));
        }
        for (index, item) in items.iter().enumerate() {
            if item.model_index as usize >= model_count {
                return Err(failure(
                    12 + index * 36 + 22,
                    format!(
                        "item={index} model={} model_count={model_count}",
                        item.model_index
                    ),
                ));
            }
        }
        let mut models = Vec::with_capacity(model_count);
        for offset in model_offsets {
            let offset = offset as usize;
            if offset < models_start || offset >= chunk_start {
                return Err(failure(offset, "model offset outside geometry region"));
            }
            models.push(parse_model(&bytes[..chunk_start], offset)?);
        }
        reader.position = chunk_start;
        let mut chunks = Vec::new();
        loop {
            let kind = reader.u32()?;
            if kind == u32::MAX {
                break;
            }
            let length = reader.u32()? as usize;
            chunks.push(Chunk {
                kind,
                payload: reader.take(length)?.to_vec(),
            });
        }
        for model in &mut models {
            model.checksum = reader.u32()?;
        }
        let count = reader.u32()? as usize;
        let texture_checksums = reader.words(count)?;
        let palettes16 = reader.palettes(16)?;
        let palettes256 = reader.palettes(256)?;
        let embedded_count = reader.u32()? as usize;
        let texture_offsets = reader.words(embedded_count)?;
        let texture_start = reader.position;
        let mut embedded_textures = Vec::with_capacity(embedded_count);
        for offset in texture_offsets {
            if (offset as usize) < texture_start {
                return Err(failure(offset as usize, "embedded texture overlaps tables"));
            }
            let mut r = Reader::new(bytes, offset as usize);
            let flags = r.u32()?;
            let color_count = r.u32()?;
            let palette_checksum = r.u32()?;
            let texture_index = r.u32()?;
            let width = r.u16()?;
            let height = r.u16()?;
            if width == 0 || height == 0 || texture_index as usize >= texture_checksums.len() {
                return Err(failure(
                    offset as usize,
                    format!("invalid texture index={texture_index} dimensions={width}x{height}"),
                ));
            }
            let row_bytes = match color_count {
                16 => ((width as usize + 3) & !3) / 2,
                256 => (width as usize + 1) & !1,
                _ => {
                    return Err(failure(
                        offset as usize + 4,
                        format!("unsupported color_count={color_count}"),
                    ));
                }
            };
            let packed = r.table(height as usize, row_bytes)?;
            let mut indices = Vec::with_capacity(width as usize * height as usize);
            for row in packed.chunks_exact(row_bytes) {
                for x in 0..width as usize {
                    let index = if color_count == 16 {
                        (row[x / 2] >> ((x & 1) * 4)) & 15
                    } else {
                        row[x]
                    };
                    indices.push(index);
                }
            }
            reader.position = reader.position.max(r.position);
            embedded_textures.push(EmbeddedTexture {
                flags,
                color_count,
                palette_checksum,
                texture_index,
                width,
                height,
                indices,
            });
        }
        let aligned_texture_end =
            embedded_count != 0 && reader.position.next_multiple_of(4) == bytes.len();
        if reader.position != bytes.len() && !aligned_texture_end {
            return Err(failure(
                reader.position,
                format!(
                    "unsupported trailing bytes={}",
                    bytes.len() - reader.position
                ),
            ));
        }
        for (model_index, model) in models.iter().enumerate() {
            for (face_index, face) in model.faces.iter().enumerate() {
                if let Some(index) = face.texture_index
                    && index as usize >= texture_checksums.len()
                {
                    return Err(failure(
                        chunk_start,
                        format!(
                            "model={model_index} face={face_index} texture_index={index} texture_count={}",
                            texture_checksums.len()
                        ),
                    ));
                }
            }
        }
        Ok(Self {
            header_word,
            items,
            models,
            chunks,
            texture_checksums,
            palettes16,
            palettes256,
            embedded_textures,
        })
    }
}

fn parse_model(bytes: &[u8], offset: usize) -> Result<Model, String> {
    let mut reader = Reader::new(bytes, offset);
    let header: [u8; 28] = reader.take(28)?.try_into().unwrap();
    let mut counts = Reader::new(&header, 2);
    let vertex_count = counts.u16()? as usize;
    let normal_count = counts.u16()? as usize;
    let face_count = counts.u16()? as usize;
    let mut vertices = Vec::with_capacity(vertex_count);
    for record in reader.table(vertex_count, 8)?.as_chunks::<8>().0 {
        let mut r = Reader::new(record, 0);
        vertices.push(Vertex {
            words: [r.u16()? as i16, r.u16()? as i16, r.u16()? as i16],
            flags: r.u16()?,
        });
    }
    let normals_raw = reader.table(normal_count, 8)?.as_chunks::<8>().0.to_vec();
    let mut faces = Vec::new();
    for _ in 0..face_count {
        let start = reader.position;
        let flags = reader.u32()?;
        let length = ((flags >> 18) * 4) as usize;
        if length < 8 || (flags & 1 != 0 && length < 20) {
            return Err(failure(
                start,
                format!("invalid face flags={flags:#x} length={length}"),
            ));
        }
        reader.position = start;
        let raw = reader.take(length)?.to_vec();
        let count = if flags & 0x10 != 0 { 3 } else { 4 };
        let indices = raw[4..4 + count].to_vec();
        if let Some(index) = indices
            .iter()
            .find(|&&index| index as usize >= vertex_count)
        {
            return Err(failure(
                start + 4,
                format!("vertex_index={index} vertex_count={vertex_count}"),
            ));
        }
        let texture_index = if flags & 1 != 0 {
            Some(u32::from_le_bytes(raw[16..20].try_into().unwrap()))
        } else {
            None
        };
        faces.push(Face {
            flags,
            indices,
            texture_index,
            raw,
        });
    }
    Ok(Model {
        header,
        vertices,
        normals_raw,
        faces,
        checksum: 0,
    })
}

struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8], position: usize) -> Self {
        Self { bytes, position }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], String> {
        let start = self.position;
        let end = start
            .checked_add(length)
            .ok_or_else(|| failure(start, "range overflow"))?;
        let result = self.bytes.get(start..end).ok_or_else(|| {
            failure(
                start,
                format!("length={length} file_size={}", self.bytes.len()),
            )
        })?;
        self.position = end;
        Ok(result)
    }

    fn table(&mut self, count: usize, stride: usize) -> Result<&'a [u8], String> {
        let length = count
            .checked_mul(stride)
            .ok_or_else(|| failure(self.position, "table size overflow"))?;
        self.take(length)
    }

    fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }

    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn words(&mut self, count: usize) -> Result<Vec<u32>, String> {
        Ok(self
            .table(count, 4)?
            .as_chunks::<4>()
            .0
            .iter()
            .map(|r| u32::from_le_bytes(*r))
            .collect())
    }

    fn palettes(&mut self, colors: usize) -> Result<Vec<Palette>, String> {
        let count = self.u32()? as usize;
        let records = self.table(count, 4 + colors * 2)?;
        records
            .chunks_exact(4 + colors * 2)
            .map(|record| {
                let mut r = Reader::new(record, 0);
                let checksum = r.u32()?;
                let colors = (0..colors)
                    .map(|_| r.u16())
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Palette { checksum, colors })
            })
            .collect()
    }
}

fn failure(offset: usize, reason: impl std::fmt::Display) -> String {
    format!("[psx] offset={offset:#x} reason={reason}")
}
