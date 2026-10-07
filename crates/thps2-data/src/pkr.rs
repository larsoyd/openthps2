use std::{fmt, ops::Range};

#[derive(Debug)]
pub struct Error(String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

#[derive(Debug)]
pub struct Entry {
    path: String,
    compression: i32,
    decoded_size: u32,
    range: Range<usize>,
    record_offset: usize,
}

impl Entry {
    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn compression(&self) -> i32 {
        self.compression
    }

    pub fn decoded_size(&self) -> u32 {
        self.decoded_size
    }

    pub fn stored_size(&self) -> usize {
        self.range.len()
    }
}

#[derive(Debug)]
pub struct Archive {
    bytes: Vec<u8>,
    entries: Vec<Entry>,
    directory_count: usize,
    header_word: u32,
}

impl Archive {
    pub fn parse(bytes: Vec<u8>) -> Result<Self, Error> {
        slice(&bytes, 0, 16)?;
        if &bytes[..4] != b"PKR2" {
            return Err(invalid(0, "expected PKR2 magic"));
        }
        let header_word = word(&bytes, 4)?;
        let directory_count = word(&bytes, 8)? as usize;
        let file_count = word(&bytes, 12)? as usize;
        let file_table = table_end(&bytes, 16, directory_count, 40)?;
        let payload_start = table_end(&bytes, file_table, file_count, 48)?;
        let mut entries = Vec::with_capacity(file_count);
        let mut claimed = vec![false; file_count];
        for index in 0..directory_count {
            let offset = 16 + index * 40;
            let directory = name(&bytes, offset)?;
            if !directory.is_empty() && !directory.ends_with('/') {
                return Err(invalid(
                    offset,
                    format!("directory={directory:?} missing trailing slash"),
                ));
            }
            let start = word(&bytes, offset + 32)? as usize;
            let count = word(&bytes, offset + 36)? as usize;
            if start < file_table || !(start - file_table).is_multiple_of(48) {
                return Err(invalid(
                    offset + 32,
                    format!("directory={directory:?} invalid file table start={start}"),
                ));
            }
            let end = table_end(&bytes, start, count, 48)?;
            if end > payload_start {
                return Err(invalid(
                    offset + 32,
                    format!("directory={directory:?} table end={end} exceeds {payload_start}"),
                ));
            }
            for record in (start..end).step_by(48) {
                let slot = (record - file_table) / 48;
                if claimed[slot] {
                    return Err(invalid(
                        record,
                        "file record belongs to multiple directories",
                    ));
                }
                claimed[slot] = true;
                let filename = name(&bytes, record)?;
                if filename.is_empty() || filename.contains(['/', '\\']) {
                    return Err(invalid(record, format!("invalid filename={filename:?}")));
                }
                let path = format!("{directory}{filename}");
                let compression = word(&bytes, record + 32)? as i32;
                let start = word(&bytes, record + 36)? as usize;
                let decoded_size = word(&bytes, record + 40)?;
                let stored_size = word(&bytes, record + 44)? as usize;
                if start < payload_start {
                    return Err(invalid(
                        record + 36,
                        format!("entry={path:?} payload offset={start} overlaps tables"),
                    ));
                }
                slice(&bytes, start, stored_size)
                    .map_err(|e| invalid(record, format!("entry={path:?} {e}")))?;
                if compression == -2 && decoded_size as usize != stored_size {
                    return Err(invalid(
                        record + 40,
                        format!(
                            "entry={path:?} raw size={stored_size} decoded_size={decoded_size}"
                        ),
                    ));
                }
                entries.push(Entry {
                    path,
                    compression,
                    decoded_size,
                    range: start..start + stored_size,
                    record_offset: record,
                });
            }
        }
        if let Some(index) = claimed.iter().position(|used| !used) {
            return Err(invalid(
                file_table + index * 48,
                "file record has no directory",
            ));
        }
        Ok(Self {
            bytes,
            entries,
            directory_count,
            header_word,
        })
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn directory_count(&self) -> usize {
        self.directory_count
    }

    pub fn header_word(&self) -> u32 {
        self.header_word
    }

    pub fn read(&self, path: &str) -> Result<&[u8], Error> {
        let normalized = path.replace('\\', "/");
        let entry = self
            .entries
            .iter()
            .find(|entry| entry.path.eq_ignore_ascii_case(&normalized))
            .ok_or_else(|| Error(format!("[pkr] entry={path:?} reason=not_found")))?;
        self.read_entry(entry)
    }

    fn read_entry(&self, entry: &Entry) -> Result<&[u8], Error> {
        if entry.compression != -2 {
            return Err(invalid(
                entry.record_offset + 32,
                format!(
                    "entry={:?} unsupported compression={}",
                    entry.path, entry.compression
                ),
            ));
        }
        Ok(&self.bytes[entry.range.clone()])
    }
}

fn invalid(offset: usize, reason: impl fmt::Display) -> Error {
    Error(format!("[pkr] offset={offset:#x} reason={reason}"))
}

fn slice(bytes: &[u8], offset: usize, length: usize) -> Result<&[u8], Error> {
    let end = offset
        .checked_add(length)
        .ok_or_else(|| invalid(offset, "range overflow"))?;
    bytes.get(offset..end).ok_or_else(|| {
        invalid(
            offset,
            format!("length={length} archive_size={}", bytes.len()),
        )
    })
}

fn word(bytes: &[u8], offset: usize) -> Result<u32, Error> {
    Ok(u32::from_le_bytes(
        slice(bytes, offset, 4)?.try_into().unwrap(),
    ))
}

fn table_end(bytes: &[u8], offset: usize, count: usize, stride: usize) -> Result<usize, Error> {
    let length = count
        .checked_mul(stride)
        .ok_or_else(|| invalid(offset, "table size overflow"))?;
    slice(bytes, offset, length)?;
    Ok(offset + length)
}

fn name(bytes: &[u8], offset: usize) -> Result<&str, Error> {
    let field = slice(bytes, offset, 32)?;
    let end = field
        .iter()
        .position(|b| *b == 0)
        .ok_or_else(|| invalid(offset, "unterminated name"))?;
    let field = &field[..end];
    if !field.iter().all(|b| b.is_ascii_graphic()) {
        return Err(invalid(offset, "unsupported non-ASCII or whitespace name"));
    }
    Ok(std::str::from_utf8(field).unwrap())
}
