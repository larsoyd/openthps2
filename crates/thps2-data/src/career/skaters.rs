use super::Skater;

/// The exe skater table: the pros, four custom slots, then the secret skaters.
pub const SKATERS: [Skater; 20] = [
    Skater::new("Tony Hawk", [7, 5, 2, 6, 8, 5, 3, 3, 6, 5], true, 0x208),
    Skater::new("Bob Burnquist", [6, 5, 5, 5, 5, 3, 8, 2, 7, 4], true, 0x0),
    Skater::new("Steve Caballero", [6, 5, 5, 6, 3, 5, 5, 6, 5, 4], true, 0x0),
    Skater::new("Kareem Campbell", [7, 2, 6, 5, 8, 4, 5, 6, 2, 5], true, 0x0),
    Skater::new("Rune Glifberg", [7, 7, 5, 6, 5, 3, 4, 4, 6, 3], true, 0x0),
    Skater::new("Eric Koston", [4, 3, 7, 5, 4, 4, 7, 7, 3, 6], true, 0x0),
    Skater::new("Bucky Lasek", [7, 7, 3, 6, 7, 3, 5, 3, 7, 2], true, 0x0),
    Skater::new("Rodney Mullen", [2, 2, 6, 5, 8, 2, 8, 7, 2, 8], true, 0x0),
    Skater::new("Chad Muska", [4, 3, 8, 6, 4, 7, 5, 8, 3, 2], true, 0x0),
    Skater::new("Andrew Reynolds", [4, 2, 8, 4, 5, 7, 5, 8, 4, 3], true, 0x0),
    Skater::new("Geoff Rowley", [6, 2, 6, 4, 5, 5, 4, 8, 7, 3], true, 0x0),
    Skater::new("Elissa Steamer", [6, 4, 5, 5, 5, 4, 5, 6, 5, 5], true, 0x1),
    Skater::new("Jamie Thomas", [4, 3, 7, 6, 4, 8, 4, 7, 4, 3], true, 0x0),
    Skater::new("Custom Skater 1", [5, 4, 5, 5, 4, 4, 4, 5, 4, 5], true, 0x2),
    Skater::new("Custom Skater 2", [5, 4, 5, 5, 4, 4, 4, 5, 4, 5], true, 0x2),
    Skater::new("Custom Skater 3", [5, 4, 5, 5, 4, 4, 4, 5, 4, 5], true, 0x2),
    Skater::new("Custom Skater 4", [5, 4, 5, 5, 4, 4, 4, 5, 4, 5], true, 0x2),
    Skater::new("Officer Dick", [6, 6, 5, 7, 5, 4, 5, 4, 4, 4], false, 0x60),
    Skater::new(
        "Private Carrera",
        [5, 5, 4, 5, 4, 4, 4, 6, 6, 7],
        false,
        0xa1,
    ),
    Skater::new("Spider-Man", [7, 7, 5, 5, 5, 4, 4, 6, 3, 4], false, 0x120),
];

/// Initials Career_Init draws for the default high scores.
pub const HIGH_SCORE_INITIALS: [&str; 39] = [
    "RJM", "RAD", "JKU", "JKS", "JGA", "MIK", "SIL", "SMP", "JOW", "ECF", "ARS", "DDT", "NSN",
    "DAB", "GAJ", "NIC", "NAH", "CJR", "AJC", "RED", "JAF", "NVF", "TMJ", "JRS", "CRA", "EAB",
    "DRM", "AJG", "HSM", "JLS", "IAN", "ALO", "LEC", "BDC", "MST", "DJA", "NF ", "FOO", "DAV",
];
