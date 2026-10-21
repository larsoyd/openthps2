/// A PRE bundle: a count, then per file a NUL terminated name and a size, each part padded to four.
#[derive(Debug)]
pub struct Pre {
    pub entries: Vec<(String, Vec<u8>)>,
}

impl Pre {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let take = |at: usize, len: usize| {
            bytes
                .get(at..at + len)
                .ok_or_else(|| format!("[pre] truncated at={at} need={len} size={}", bytes.len()))
        };
        let word =
            |at: usize| take(at, 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize);
        let count = word(0)?;
        let mut at = 4;
        let mut entries = Vec::with_capacity(count.min(4096));
        for i in 0..count {
            let rest = bytes.get(at..).unwrap_or_default();
            let len = rest
                .iter()
                .position(|&b| b == 0)
                .ok_or_else(|| format!("[pre] entry={i} name unterminated at={at}"))?;
            let name = String::from_utf8_lossy(&rest[..len]).into_owned();
            at += (len + 1).next_multiple_of(4);
            let size = word(at)?;
            let data = take(at + 4, size)?.to_vec();
            at += 4 + size.next_multiple_of(4);
            entries.push((name, data));
        }
        if at != bytes.len() {
            return Err(format!("[pre] size={} expected={at}", bytes.len()));
        }
        Ok(Self { entries })
    }

    pub fn get(&self, name: &str) -> Option<&[u8]> {
        self.entries
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, b)| b.as_slice())
    }
}
