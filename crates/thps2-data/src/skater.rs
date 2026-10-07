use crate::collision::{CollisionWorld, Hit};

pub const STANDING_HEIGHT: i32 = 0x1e;
pub const GROUND_PROBE_ABOVE: i32 = 0x46;
pub const GROUND_PROBE_SPAN: i32 = 0x100;
pub const GROUND_STICK_DISTANCE: i32 = 0x74;

/// Native a*b/4096 through x87 then truncated toward zero.
pub fn mul_q12(a: i32, b: i32) -> i32 {
    (i64::from(a) * i64::from(b) / 4096) as i32
}

/// Native dot product with Q12 scaling, truncated toward zero.
pub fn dot_q12(a: [i32; 3], b: [i32; 3]) -> i32 {
    let sum: i128 = (0..3).map(|i| i128::from(a[i]) * i128::from(b[i])).sum();
    (sum / 4096) as i32
}

pub fn sqrt_int(value: i32) -> i32 {
    if value <= 0 {
        return 0;
    }
    (f64::from(value)).sqrt() as i32
}

/// Halves components until they fit the native short range, then scales to a Q12 unit vector.
pub fn normalize(mut v: [i32; 3]) -> [i32; 3] {
    while v.iter().any(|&c| !(-0x6882..=0x6882).contains(&c)) {
        v = v.map(|c| c >> 1);
    }
    let length = sqrt_int(v.iter().map(|&c| c * c).sum());
    if length == 0 {
        return [0x1000, 0, 0];
    }
    v.map(|c| (c << 12) / length)
}

/// Native move retry order after a blocked segment, ending with no movement at all.
pub fn move_retrying<E>(
    from: [i32; 3],
    to: [i32; 3],
    mut blocked: impl FnMut([i32; 3], [i32; 3]) -> Result<bool, E>,
) -> Result<[i32; 3], E> {
    let [px, py, pz] = from;
    let [nx, ny, nz] = to;
    let candidates = [
        to,
        [px, ny, nz],
        [nx, ny, pz],
        [nx, py, nz],
        [nx, py, pz],
        [px, py, nz],
        [px, ny, pz],
    ];
    for candidate in candidates {
        if !blocked(from, candidate)? {
            return Ok(candidate);
        }
    }
    Ok(from)
}

fn offset(position: [i32; 3], up: [i32; 3], units: i32) -> Result<[i32; 3], String> {
    let mut out = [0; 3];
    for axis in 0..3 {
        out[axis] = up[axis]
            .checked_mul(units)
            .and_then(|v| position[axis].checked_add(v))
            .ok_or("skater offset overflow")?;
    }
    Ok(out)
}

impl CollisionWorld {
    pub fn move_retrying(&self, from: [i32; 3], to: [i32; 3]) -> Result<[i32; 3], String> {
        move_retrying(from, to, |start, end| Ok(self.query(start, end)?.is_some()))
    }

    /// Probes 70 units above the skater to 186 below along its Q12 up vector.
    pub fn ground_probe(&self, position: [i32; 3], up: [i32; 3]) -> Result<Option<Hit>, String> {
        let start = offset(position, up, GROUND_PROBE_ABOVE)?;
        let end = offset(start, up, -GROUND_PROBE_SPAN)?;
        self.query(start, end)
    }

    /// Moves to standing height above the hit and keeps the old position for sub-unit corrections.
    pub fn stand(&self, position: [i32; 3], up: [i32; 3], hit: &Hit) -> Result<[i32; 3], String> {
        let target = offset(hit.position_fixed, up, STANDING_HEIGHT)?;
        let moved = self.move_retrying(position, target)?;
        let delta: [i32; 3] = std::array::from_fn(|a| {
            position[a]
                .checked_sub(moved[a])
                .expect("sub-unit stand delta fits")
        });
        if dot_q12(delta, delta) < 0x1000 {
            return Ok(position);
        }
        Ok(moved)
    }
}
