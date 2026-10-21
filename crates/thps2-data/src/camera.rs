use crate::ground::trig_q12;

pub fn rsin(angle: i32) -> i32 {
    trig_q12(angle).1
}

pub fn rcos(angle: i32) -> i32 {
    trig_q12(angle).0
}

/// Arc cosine of a Q12 value in 4096 units per turn.
pub fn racos(x: i32) -> i32 {
    ((f64::from(x) * (1.0 / 4096.0)).acos() * 4096.0 * 0.159_154_943_091_895_35) as i32
}
