/// Slot table: the moves a skater's trick slots fill, with their category mask and emit rank.
pub const SLOTS: [super::Slot; 30] = [
    super::Slot {
        buttons: &[1, 12],
        mask: 0x4000,
        rank: 0,
    },
    super::Slot {
        buttons: &[4, 12],
        mask: 0x4000,
        rank: 0,
    },
    super::Slot {
        buttons: &[2, 12],
        mask: 0x4000,
        rank: 0,
    },
    super::Slot {
        buttons: &[3, 12],
        mask: 0x4000,
        rank: 0,
    },
    super::Slot {
        buttons: &[1, 1, 12],
        mask: 0x800,
        rank: 0,
    },
    super::Slot {
        buttons: &[2, 2, 12],
        mask: 0x800,
        rank: 0,
    },
    super::Slot {
        buttons: &[1, 10],
        mask: 0x300,
        rank: 3,
    },
    super::Slot {
        buttons: &[6, 10],
        mask: 0x300,
        rank: 2,
    },
    super::Slot {
        buttons: &[4, 10],
        mask: 0x300,
        rank: 3,
    },
    super::Slot {
        buttons: &[8, 10],
        mask: 0x300,
        rank: 2,
    },
    super::Slot {
        buttons: &[2, 10],
        mask: 0x300,
        rank: 3,
    },
    super::Slot {
        buttons: &[7, 10],
        mask: 0x300,
        rank: 2,
    },
    super::Slot {
        buttons: &[3, 10],
        mask: 0x300,
        rank: 3,
    },
    super::Slot {
        buttons: &[5, 10],
        mask: 0x300,
        rank: 2,
    },
    super::Slot {
        buttons: &[1, 9],
        mask: 0x300,
        rank: 3,
    },
    super::Slot {
        buttons: &[6, 9],
        mask: 0x300,
        rank: 2,
    },
    super::Slot {
        buttons: &[4, 9],
        mask: 0x300,
        rank: 3,
    },
    super::Slot {
        buttons: &[8, 9],
        mask: 0x300,
        rank: 2,
    },
    super::Slot {
        buttons: &[2, 9],
        mask: 0x300,
        rank: 3,
    },
    super::Slot {
        buttons: &[7, 9],
        mask: 0x300,
        rank: 2,
    },
    super::Slot {
        buttons: &[3, 9],
        mask: 0x300,
        rank: 3,
    },
    super::Slot {
        buttons: &[5, 9],
        mask: 0x300,
        rank: 2,
    },
    super::Slot {
        buttons: &[1, 1, 10],
        mask: 0x300,
        rank: 1,
    },
    super::Slot {
        buttons: &[4, 4, 10],
        mask: 0x300,
        rank: 1,
    },
    super::Slot {
        buttons: &[2, 2, 10],
        mask: 0x300,
        rank: 1,
    },
    super::Slot {
        buttons: &[3, 3, 10],
        mask: 0x300,
        rank: 1,
    },
    super::Slot {
        buttons: &[1, 1, 9],
        mask: 0x300,
        rank: 1,
    },
    super::Slot {
        buttons: &[4, 4, 9],
        mask: 0x300,
        rank: 1,
    },
    super::Slot {
        buttons: &[2, 2, 9],
        mask: 0x300,
        rank: 1,
    },
    super::Slot {
        buttons: &[3, 3, 9],
        mask: 0x300,
        rank: 1,
    },
];

/// Button sequences a special trick can be put on.
pub const COMBOS: [&[u8]; 36] = [
    &[1, 4, 12],
    &[1, 2, 12],
    &[1, 3, 12],
    &[4, 1, 12],
    &[4, 2, 12],
    &[4, 3, 12],
    &[2, 1, 12],
    &[2, 4, 12],
    &[2, 3, 12],
    &[3, 1, 12],
    &[3, 4, 12],
    &[3, 2, 12],
    &[1, 4, 10],
    &[1, 2, 10],
    &[1, 3, 10],
    &[4, 1, 10],
    &[4, 2, 10],
    &[4, 3, 10],
    &[2, 1, 10],
    &[2, 4, 10],
    &[2, 3, 10],
    &[3, 1, 10],
    &[3, 4, 10],
    &[3, 2, 10],
    &[1, 4, 9],
    &[1, 2, 9],
    &[1, 3, 9],
    &[4, 1, 9],
    &[4, 2, 9],
    &[4, 3, 9],
    &[2, 1, 9],
    &[2, 4, 9],
    &[2, 3, 9],
    &[3, 1, 9],
    &[3, 4, 9],
    &[3, 2, 9],
];

/// Script op size class by op minus one, as the trick database scanner reads it.
pub const OP_SIZE_INDEX: [u8; 0x59] = [
    0, 1, 1, 9, 9, 9, 1, 9, 0, 0, 2, 1, 0, 0, 3, 0, 0, 0, 1, 1, 0, 1, 4, 0, 0, 1, 0, 1, 0, 1, 0, 1,
    0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 3, 1, 0, 0, 0, 1, 1, 1, 1, 0, 1, 0, 4, 3, 5, 0, 1, 1, 0, 1, 0, 0,
    0, 0, 0, 0, 3, 1, 4, 4, 0, 1, 0, 6, 1, 0, 7, 2, 0, 8, 0, 0, 8, 1, 1, 0, 8,
];
