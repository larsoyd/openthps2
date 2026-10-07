#[derive(Debug)]
pub struct Node {
    pub kind: i16,
    pub raw: Vec<u8>,
}

#[derive(Debug)]
pub struct Restart {
    pub node_index: usize,
    pub links: Vec<u16>,
    pub position_fixed: [i32; 3],
    pub angles: [i16; 3],
    pub name: String,
    pub script: Vec<u8>,
}

#[derive(Debug)]
pub struct Triggers {
    pub nodes: Vec<Node>,
    pub restarts: Vec<Restart>,
    pub header_reserved: u16,
}

fn bytes<const N: usize>(data: &[u8], offset: usize) -> Result<[u8; N], String> {
    data.get(offset..offset.saturating_add(N))
        .and_then(|v| v.try_into().ok())
        .ok_or_else(|| format!("trigger offset={offset} truncated record"))
}

impl Triggers {
    pub fn parse(data: &[u8]) -> Result<Self, String> {
        if bytes::<4>(data, 0)? != *b"_TRG" || u32::from_le_bytes(bytes(data, 4)?) != 2 {
            return Err("trigger unsupported magic/version".into());
        }
        let count = u16::from_le_bytes(bytes(data, 8)?) as usize;
        let header_reserved = u16::from_le_bytes(bytes(data, 10)?);
        if count == 0 {
            return Err("trigger missing terminator node".into());
        }
        let table_end = 12 + count * 4;
        let mut offsets = Vec::with_capacity(count);
        for index in 0..count {
            let offset = u32::from_le_bytes(bytes(data, 12 + index * 4)?) as usize;
            if offset < table_end
                || !offset.is_multiple_of(2)
                || offset.saturating_add(2) > data.len()
            {
                return Err(format!("trigger node={index} invalid offset={offset}"));
            }
            offsets.push(offset);
        }
        let mut boundaries = offsets.clone();
        boundaries.push(data.len());
        boundaries.sort_unstable();
        boundaries.dedup();
        let mut nodes = Vec::with_capacity(count);
        let mut restarts = Vec::new();
        for (node_index, &offset) in offsets.iter().enumerate() {
            let end = boundaries[boundaries.binary_search(&offset).unwrap() + 1];
            let raw = &data[offset..end];
            let kind = i16::from_le_bytes(bytes(raw, 0)?);
            if kind == 8 {
                let link_count = u16::from_le_bytes(bytes(raw, 2)?) as usize;
                let mut links = Vec::with_capacity(link_count);
                for index in 0..link_count {
                    let link = u16::from_le_bytes(bytes(raw, 4 + index * 2)?);
                    if link as usize >= count {
                        return Err(format!("trigger node={node_index} invalid link={link}"));
                    }
                    links.push(link);
                }
                let position_offset = ((offset + 4 + link_count * 2 + 3) & !3) - offset;
                let mut position_fixed = [0; 3];
                let mut angles = [0; 3];
                for axis in 0..3 {
                    position_fixed[axis] =
                        i32::from_le_bytes(bytes(raw, position_offset + axis * 4)?)
                            .checked_mul(4096)
                            .ok_or_else(|| {
                                format!("trigger node={node_index} position overflow")
                            })?;
                    angles[axis] = i16::from_le_bytes(bytes(raw, position_offset + 12 + axis * 2)?);
                }
                let name_start = position_offset + 18;
                let name_bytes = raw
                    .get(name_start..)
                    .ok_or("trigger truncated restart name")?;
                let length = name_bytes
                    .iter()
                    .position(|&v| v == 0)
                    .ok_or("trigger unterminated restart name")?;
                if !name_bytes[..length].is_ascii() {
                    return Err("trigger non-ASCII restart name unsupported".into());
                }
                let name = String::from_utf8(name_bytes[..length].to_vec()).unwrap();
                let script_start = ((offset + name_start + length + 2) & !1) - offset;
                let script = raw
                    .get(script_start..)
                    .ok_or("trigger truncated restart alignment")?
                    .to_vec();
                restarts.push(Restart {
                    node_index,
                    links,
                    position_fixed,
                    angles,
                    name,
                    script,
                });
                if restarts.len() > 40 {
                    return Err("trigger more than 40 restarts unsupported".into());
                }
            }
            nodes.push(Node {
                kind,
                raw: raw.to_vec(),
            });
        }
        if nodes.last().unwrap().kind != 255 {
            return Err("trigger last node must be terminator255".into());
        }
        Ok(Self {
            nodes,
            restarts,
            header_reserved,
        })
    }

    pub fn restart_named(&self, name: &str) -> Result<&Restart, String> {
        self.restarts
            .iter()
            .find(|r| r.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| format!("restart name={name:?} not found"))
    }
}
