/// Deliberate departures from the original, all on by default. Comparisons with the original turn
/// one off to show a difference comes from that fix alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fixes {
    /// The pending multiplier shows the factor the landing applies. The original reads its table
    /// at twice the trick count, so from eight tricks on it shows less than it scores.
    pub multiplier_text: bool,
}

impl Fixes {
    pub const ALL: Self = Self {
        multiplier_text: true,
    };
    pub const ORIGINAL: Self = Self {
        multiplier_text: false,
    };
}

impl Default for Fixes {
    fn default() -> Self {
        Self::ALL
    }
}
