/// The game's random number generator, three words stepped together on every call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rng {
    s: u32,
    a: u32,
    b: u32,
}

impl Rng {
    /// The words before any seed, zero like the rest of the bss.
    pub fn unseeded() -> Self {
        Self { s: 0, a: 0, b: 0 }
    }

    pub fn seeded(seed: u32) -> Self {
        Self {
            s: seed,
            a: 0x12b9b0a1,
            b: 0x0aa2fb3f,
        }
    }

    pub fn state(&self) -> [u32; 3] {
        [self.s, self.a, self.b]
    }

    pub fn set_state(&mut self, [s, a, b]: [u32; 3]) {
        (self.s, self.a, self.b) = (s, a, b);
    }

    /// A value in 0..n from the low half of the new state.
    pub fn next(&mut self, n: i32) -> i32 {
        self.s = self.a.wrapping_mul(self.s).wrapping_add(self.b);
        let t = self.s as i32;
        self.a = (self.a ^ self.s).wrapping_add((t >> 4) as u32);
        self.b = self
            .b
            .wrapping_sub(0x10101010)
            .wrapping_add((t >> 3) as u32);
        ((self.s & 0xffff) as i32).wrapping_mul(n) >> 16
    }
}
