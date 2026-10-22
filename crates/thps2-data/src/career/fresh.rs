use super::{
    BLOCK_SIZE, CAREER_VERSION, CHARACTERS, CareerSave, HIGH_SCORE_INITIALS, LEVELS, PROGRESS_SIZE,
    Skater,
};
use crate::create::{CUSTOM_SIZE, CUSTOM_SKATERS, TextureSets, custom_defaults};
use crate::input::{TrickConfig, known_tricks};
use crate::rng::Rng;
use crate::script::TrickScripts;

/// The boot seed, the generator then runs through the custom skater looks before New Career.
const BOOT_SEED: u32 = 0x12a6cc58;
pub(super) const HIGH_SCORES: usize = 0x1454;
pub(super) const HIGH_SCORE_SIZE: usize = 8;
pub(super) const HIGH_SCORE_ENTRIES: usize = 8;
const PLAYER_DECKS: usize = 0x1878;
const VOLUMES: usize = 0x1880;
/// Career_Init sets this byte to 100, no reader is known.
const INIT_100: usize = 0x192e;
const PAD_CONFIG: usize = 0x1950;
const PAD_FLAGS: usize = 0x1970;
pub(super) const INITIALS: usize = 0x19a0;
pub(super) const TIPS_SHOWN: usize = 0x19a4;
pub(super) const TIPS_ON: usize = 0x1a04;
const HORSE_WORD: usize = 0x1a10;
const CUSTOM_SKATER_BLOCK: usize = 0x1a20;
const CUSTOM_TRICKS: usize = 0x1c28;
const CUSTOM_FLAG: u32 = 2;
const SECRET: u32 = 0x20;

impl CareerSave {
    /// New Career after a boot: Career_Init over the custom skater looks the boot front end
    /// drew, with the front end and option defaults, and the skater table as unlocks left it.
    pub fn fresh(
        scripts: &TrickScripts,
        sets: &TextureSets,
        skaters: &[Skater; CHARACTERS],
    ) -> Result<Self, String> {
        let mut rng = Rng::seeded(BOOT_SEED);
        let mut b = vec![0; BLOCK_SIZE];
        for slot in 0..CUSTOM_SKATERS {
            let at = CUSTOM_SKATER_BLOCK + CUSTOM_SIZE * slot;
            b[at..at + CUSTOM_SIZE].copy_from_slice(&custom_defaults(slot, sets, &mut rng)?);
        }
        b[..4].copy_from_slice(&CAREER_VERSION.to_le_bytes());
        high_scores(&mut b, &mut rng, skaters);
        for (id, skater) in skaters.iter().enumerate() {
            let at = 4 + PROGRESS_SIZE * id;
            b[at..at + PROGRESS_SIZE].copy_from_slice(&fresh_record(scripts, skater, id)?);
        }
        b[CUSTOM_TRICKS..CUSTOM_TRICKS + 0x35]
            .copy_from_slice(&TrickConfig::custom(scripts)?.to_bytes());
        front_end_defaults(&mut b);
        Ok(Self {
            name: String::new(),
            block: b,
        })
    }
}

/// Per level, eight entries of initials drawn from the default list and a skater drawn until
/// one is selectable and neither custom nor hidden.
fn high_scores(b: &mut [u8], rng: &mut Rng, skaters: &[Skater]) {
    for level in 0..LEVELS.len() {
        for entry in 0..HIGH_SCORE_ENTRIES {
            let at = HIGH_SCORES + HIGH_SCORE_SIZE * (HIGH_SCORE_ENTRIES * level + entry);
            let name = HIGH_SCORE_INITIALS[rng.next(HIGH_SCORE_INITIALS.len() as i32) as usize];
            b[at..at + 3].copy_from_slice(name.as_bytes());
            let score = (8 + 4 * level as u32 - entry as u32) * 5000;
            b[at + 4..at + 8].copy_from_slice(&score.to_le_bytes());
            let skater = loop {
                let k = rng.next(20) as usize;
                let s = &skaters[k];
                if s.selectable && s.flags & (CUSTOM_FLAG | SECRET) == 0 {
                    break k;
                }
            };
            b[at + 3] = skater as u8;
        }
    }
}

/// A character record as InitProgress leaves it: stats, the deck mask, the list's tricks bought
/// and its default trick slots.
/// Custom skaters would add their bought stat points, a boot look has none.
fn fresh_record(
    scripts: &TrickScripts,
    skater: &Skater,
    id: usize,
) -> Result<[u8; PROGRESS_SIZE], String> {
    let mut r = [0; PROGRESS_SIZE];
    r[0x38..0x42].copy_from_slice(&skater.stats);
    let mask: u32 = if skater.flags & CUSTOM_FLAG != 0 {
        0x3ff
    } else {
        1
    };
    r[0x34..0x38].copy_from_slice(&mask.to_le_bytes());
    for (i, w) in known_tricks(scripts, id)?.iter().enumerate() {
        r[0xac + 4 * i..0xb0 + 4 * i].copy_from_slice(&w.to_le_bytes());
    }
    r[0xcc..0x101].copy_from_slice(&TrickConfig::defaults(scripts, id)?.to_bytes());
    Ok(r)
}

/// The front end state Career_Init copies in after the boot defaults: both players on Tony
/// Hawk's first deck, volumes 12 and 6, pad layouts, trick tips on and the HORSE word.
fn front_end_defaults(b: &mut [u8]) {
    let words = |b: &mut [u8], at: usize, values: &[u32]| {
        for (i, v) in values.iter().enumerate() {
            b[at + 4 * i..at + 4 * i + 4].copy_from_slice(&v.to_le_bytes());
        }
    };
    b[PLAYER_DECKS..PLAYER_DECKS + 2].fill(1);
    words(b, VOLUMES, &[12, 6]);
    b[INIT_100] = 100;
    words(b, PAD_CONFIG, &[0, 1, 2, 3, 0, 1, 2, 3]);
    words(b, PAD_FLAGS, &[1, 1, 1, 1]);
    b[INITIALS..INITIALS + 3].copy_from_slice(b"AAA");
    words(b, TIPS_ON, &[1, 1, 5]);
    b[HORSE_WORD..HORSE_WORD + 5].copy_from_slice(b"HORSE");
}
