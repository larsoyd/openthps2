use crate::rng::Rng;

const MAX_CLOUDS: usize = 12;

/// Which event started the burst, each with its own terrain table.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DustKind {
    #[default]
    Land,
    Bail,
    Brake,
}

/// A skater's dust: a landing, a bail or the slide brake restarts the burst on the ground's
/// terrain and each frame of it adds a cloud. Only the clouds alive and the draws are kept,
/// their colour and size only matter to drawing them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Dust {
    kind: DustKind,
    countdown: i32,
    clouds: usize,
}

fn burst_clouds(kind: DustKind, terrain: u32) -> i32 {
    match (kind, terrain) {
        (DustKind::Land, 3 | 0xe) => 8,
        (DustKind::Land, 6) => 12,
        (DustKind::Bail, 3 | 5 | 8 | 0xd | 0xe) => 10,
        (DustKind::Brake, 3 | 0xe) => 10,
        (DustKind::Brake, 8 | 0xd) => 6,
        _ => 0,
    }
}

impl Dust {
    pub fn reset(&mut self, kind: DustKind, terrain: u32) {
        self.kind = kind;
        self.countdown = burst_clouds(kind, terrain);
    }

    pub fn clouds(&self) -> usize {
        self.clouds
    }

    /// Every cloud drifts up by a draw, then the burst adds one, a bail cloud drawing its
    /// spread and size and the others two offsets. The release of faded clouds tests the colour
    /// word with its flag byte, so they are never released and a thirteenth replaces the dimmest.
    pub fn update(&mut self, rng: &mut Rng) {
        for _ in 0..self.clouds {
            rng.next(3);
        }
        if self.countdown < 1 {
            return;
        }
        self.countdown -= 1;
        let draws = if self.kind == DustKind::Bail { 3 } else { 2 };
        for _ in 0..draws {
            rng.next(1);
        }
        self.clouds = (self.clouds + 1).min(MAX_CLOUDS);
    }
}
