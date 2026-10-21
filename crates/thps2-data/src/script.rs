const BAIL_KINDS: usize = 25;
const GRIND_TRICKS: usize = 0xd0;

/// The trick script file: bytecode the skater runs for tricks, manuals and bails.
pub struct TrickScripts {
    data: Vec<u8>,
    bail_table: usize,
}

impl TrickScripts {
    pub fn parse(data: Vec<u8>) -> Result<Self, String> {
        let bail_table = usize::from(u16::from_le_bytes(
            data.get(4..6)
                .ok_or("trick scripts truncated header")?
                .try_into()
                .unwrap(),
        ));
        if bail_table + BAIL_KINDS * 2 > data.len() {
            return Err(format!(
                "trick scripts bail table={bail_table:#x} past end len={:#x}",
                data.len()
            ));
        }
        Ok(Self { data, bail_table })
    }

    pub fn bail_script(&self, kind: usize) -> Result<usize, String> {
        if kind >= BAIL_KINDS {
            return Err(format!("bail kind={kind} outside the table"));
        }
        self.offset(self.bail_table + kind * 2)
    }

    /// Grind script by approach and pad code, from the table the header's second word points at.
    pub fn grind_script(&self, index: usize) -> Result<usize, String> {
        if index >= GRIND_TRICKS {
            return Err(format!("grind trick index={index} outside the table"));
        }
        let table = usize::from(u16::from_le_bytes([self.data[2], self.data[3]]));
        self.offset(table + index * 2)
    }

    /// Header word i, a table offset from the start of the file.
    pub fn header(&self, i: usize) -> Result<usize, String> {
        self.offset(i * 2)
    }

    pub fn byte(&self, at: usize) -> Option<u8> {
        self.data.get(at).copied()
    }

    pub fn short(&self, at: usize) -> Option<i16> {
        Some(i16::from_le_bytes(
            self.data.get(at..at + 2)?.try_into().ok()?,
        ))
    }

    /// Reads a script offset, which the game sign extends from sixteen bits.
    pub fn offset(&self, at: usize) -> Result<usize, String> {
        let raw = self
            .short(at)
            .ok_or_else(|| format!("trick script offset at={at:#x} truncated"))?;
        usize::try_from(raw)
            .ok()
            .filter(|&o| o < self.data.len())
            .ok_or_else(|| format!("trick script offset at={at:#x} value={raw} outside the file"))
    }
}
