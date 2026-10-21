use crate::{
    font::Font,
    ground::Session,
    text::{CharMap, Prim, Text},
};

const CLOCK_AT: [i32; 2] = [256, 48];
/// Below 0:11 the clock blinks off for half of every second.
const BLINK_BELOW: i32 = 660;

pub const CLOCK: Text = Text {
    hjust: 1,
    vjust: 3,
    spacing: 4,
    colour: [0x80, 0x30, 0x30],
    shadow: Some([4, 4]),
    fixed_pitch: true,
    scale: 0x1000,
    map: CharMap::Digits,
};

/// The single session clock in the time font, nothing while it blinks off. The VB counter stops
/// at the run end, which keeps it off from there in Hangar.
pub fn clock(font: &Font, session: &Session, time: i32, vb: i32) -> Result<Vec<Prim>, String> {
    if session.limit - time < BLINK_BELOW && vb.rem_euclid(60) < 30 {
        return Ok(Vec::new());
    }
    CLOCK.draw(font, CLOCK_AT[0], CLOCK_AT[1], &session.clock_text(time))
}
