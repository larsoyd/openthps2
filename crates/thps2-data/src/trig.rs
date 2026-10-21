#[derive(Debug)]
pub struct Node {
    pub kind: i16,
    /// File offset of the node, scripts align their operands to it.
    pub offset: usize,
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

/// Env items whose origin lies in (or outside on every axis of) a box get shown or hidden.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Regions {
    pub show: bool,
    pub inside: bool,
    pub boxes: Vec<[[i32; 3]; 2]>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    Gap {
        end: u32,
        id: i16,
    },
    InitialPulses(u16),
    SendPulse,
    Regions(Regions),
    /// Kills the objects, skaters or rails the node links to.
    Kill,
    /// Signals the moving objects the node links to.
    Signal,
    /// Shows or hides the env items the node links to.
    Visible(bool),
}

const COMMAND_POINT: i16 = 6;

/// An object node keeps its type at +2 and its link count at +6, then flag bytes up to 0xff, an
/// aligned origin, three angles and the script.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectNode {
    pub object_type: u16,
    pub flags: Vec<u8>,
    pub script: Vec<u16>,
}

impl Node {
    fn links(&self) -> usize {
        self.raw
            .get(2..4)
            .map_or(0, |b| usize::from(u16::from_le_bytes([b[0], b[1]])))
    }

    /// Node offset of the four byte aligned word after the links.
    fn after_links(&self) -> usize {
        ((self.offset + 4 + self.links() * 2 + 3) & !3) - self.offset
    }

    fn checksum(&self) -> Option<u32> {
        let at = self.after_links();
        Some(u32::from_le_bytes(
            self.raw.get(at..at + 4)?.try_into().ok()?,
        ))
    }
}

impl Triggers {
    pub fn command_point(&self, checksum: u32) -> Option<usize> {
        self.nodes
            .iter()
            .position(|n| n.kind == COMMAND_POINT && n.checksum() == Some(checksum))
    }

    /// Decodes a command point script, refusing opcodes the rewrite does not run.
    pub fn commands(&self, node: usize) -> Result<Vec<Command>, String> {
        let n = &self.nodes[node];
        if n.kind != COMMAND_POINT {
            return Err(format!(
                "trigger node={node} kind={} is no command point",
                n.kind
            ));
        }
        let half = |at: usize| {
            n.raw
                .get(at..at + 2)
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                .ok_or_else(|| format!("trigger node={node} script truncated at={at}"))
        };
        let mut at = n.after_links() + 4;
        let mut commands = Vec::new();
        loop {
            let op = half(at)?;
            at += 2;
            match op {
                0xffff => return Ok(commands),
                0xc9 => {
                    at = ((n.offset + at + 3) & !3) - n.offset;
                    let end = u32::from(half(at)?) | u32::from(half(at + 2)?) << 16;
                    commands.push(Command::Gap {
                        end,
                        id: half(at + 4)? as i16,
                    });
                    at += 6;
                }
                0x86 => {
                    commands.push(Command::InitialPulses(half(at)?));
                    at += 2;
                }
                0x03 => commands.push(Command::SendPulse),
                0x0c => commands.push(Command::Kill),
                0x0a => commands.push(Command::Signal),
                0x0d => {
                    commands.push(Command::Visible(half(at)? != 0));
                    at += 2;
                }
                0x8d => {
                    let (mode, inside) = (half(at)?, half(at + 2)? != 0);
                    at += 4;
                    let mut boxes = Vec::new();
                    while half(at)? != 0xff {
                        at = ((n.offset + at + 3) & !3) - n.offset;
                        let word = |k: usize| -> Result<i32, String> {
                            let lo = u32::from(half(at + k * 4)?);
                            Ok((lo | u32::from(half(at + k * 4 + 2)?) << 16) as i32)
                        };
                        let corner = |k: usize| -> Result<[i32; 3], String> {
                            Ok([word(k)? << 12, word(k + 1)? << 12, word(k + 2)? << 12])
                        };
                        boxes.push([corner(0)?, corner(3)?]);
                        at += 24;
                    }
                    at += 2;
                    let show = if mode <= 1 { mode != 0 } else { mode != 2 };
                    commands.push(Command::Regions(Regions {
                        show,
                        inside,
                        boxes,
                    }));
                }
                _ => {
                    return Err(format!(
                        "trigger node={node} command op={op:#x} at={at} unsupported"
                    ));
                }
            }
        }
    }

    pub fn object(&self, node: usize) -> Result<ObjectNode, String> {
        let n = &self.nodes[node];
        let half = |at: usize| {
            n.raw
                .get(at..at + 2)
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                .ok_or_else(|| format!("object node={node} truncated at={at}"))
        };
        let mut at = 8 + usize::from(half(6)?) * 2;
        let mut flags = Vec::new();
        loop {
            let flag = *n
                .raw
                .get(at)
                .ok_or_else(|| format!("object node={node} flags unterminated"))?;
            at += 1;
            if flag == 0xff {
                break;
            }
            flags.push(flag);
        }
        at = ((n.offset + at + 3) & !3) - n.offset + 18;
        let script = (at..n.raw.len() - 1)
            .step_by(2)
            .map(half)
            .collect::<Result<_, _>>()?;
        Ok(ObjectNode {
            object_type: half(2)?,
            flags,
            script,
        })
    }

    pub fn links(&self, node: usize) -> Vec<usize> {
        let n = &self.nodes[node];
        (0..n.links())
            .map(|k| usize::from(u16::from_le_bytes([n.raw[4 + k * 2], n.raw[5 + k * 2]])))
            .collect()
    }

    /// World position and flags of a rail node, flags 0xffff reading as none.
    pub fn rail_point(&self, node: usize) -> Option<([i32; 3], u16)> {
        let n = self.nodes.get(node)?;
        if n.kind != 10 && n.kind != 11 {
            return None;
        }
        let at = n.after_links();
        let word = |k: usize| {
            Some(i32::from_le_bytes(
                n.raw.get(at + k * 4..at + k * 4 + 4)?.try_into().ok()?,
            ))
        };
        let position = [word(0)? << 12, word(1)? << 12, word(2)? << 12];
        let flags = u16::from_le_bytes(n.raw.get(at + 12..at + 14)?.try_into().ok()?);
        Some((position, if flags == 0xffff { 0 } else { flags }))
    }

    pub fn kind(&self, node: usize) -> i16 {
        self.nodes[node].kind
    }

    pub fn is_command_point(&self, node: usize) -> bool {
        self.nodes[node].kind == COMMAND_POINT
    }

    pub fn checksum(&self, node: usize) -> Option<u32> {
        self.nodes[node].checksum()
    }

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
                offset,
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
