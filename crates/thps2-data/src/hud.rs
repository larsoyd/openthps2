use crate::{
    font::Font,
    ground::{Run, Session, Skater},
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

const CENTRE_X: i32 = 256;
const TITLE: [u8; 3] = [0xb6, 0x84, 0x24];
const GREY: [u8; 3] = [0x7a, 0x7a, 0x7a];
const PROMPT: [u8; 3] = [0x96, 0x64, 0x19];
const SLIDE_STEP: i32 = 30;
const LABEL_STOP: i32 = 90;
const VALUE_STOP: i32 = 410;
const ROWS: [(usize, i32); 3] = [(0, 60), (1, 78), (3, 96)];
const MENU_ITEMS: [&str; 5] = [
    "Update save file",
    "Retry",
    "Level Select",
    "Watch replay",
    "Quit",
];
const MENU_RETRY: usize = 1;
const MENU_SPACING: i32 = 22;
const MENU_SELECTED_SCALE: i32 = 0x14f;
const MENU_SCALE: i32 = 0x100;
const MENU_SCALE_STEP: i32 = 0x28;

/// The message font text object, one style for every front end line.
fn mess(hjust: u8, colour: [u8; 3], scale: i32) -> Text {
    Text {
        hjust,
        vjust: 1,
        spacing: 2,
        colour,
        shadow: Some([2, 2]),
        fixed_pitch: false,
        scale: scale << 4,
        map: CharMap::Letters,
    }
}

/// Labels slide in from the left and values from the right, one step per pass.
#[derive(Clone, Debug)]
struct Slides {
    labels: [i32; 8],
    values: [i32; 8],
}

impl Slides {
    fn new() -> Self {
        Self {
            labels: std::array::from_fn(|i| -220 + SLIDE_STEP * i as i32),
            values: std::array::from_fn(|i| 732 - SLIDE_STEP * i as i32),
        }
    }

    fn step(&mut self) {
        for i in 0..5 {
            self.labels[i] = (self.labels[i] + SLIDE_STEP).min(LABEL_STOP);
            self.values[i] = (self.values[i] - SLIDE_STEP).max(VALUE_STOP);
        }
    }
}

#[derive(Clone, Debug)]
struct EndMenu {
    selected: usize,
    scales: [i32; 5],
}

impl EndMenu {
    fn new() -> Self {
        Self {
            selected: MENU_RETRY,
            scales: std::array::from_fn(|i| Self::target(i, MENU_RETRY)),
        }
    }

    fn target(item: usize, selected: usize) -> i32 {
        if item == selected {
            MENU_SELECTED_SCALE
        } else {
            MENU_SCALE
        }
    }

    fn step(&mut self) {
        for (i, scale) in self.scales.iter_mut().enumerate() {
            let target = Self::target(i, self.selected);
            *scale += (target - *scale).clamp(-MENU_SCALE_STEP, MENU_SCALE_STEP);
        }
    }

    /// The selected item pulses toward orange on the VB tick count.
    fn draw(&self, font: &Font, ticks: i32) -> Result<Vec<Prim>, String> {
        let top = (240 - (MENU_ITEMS.len() as i32 - 1) * MENU_SPACING) / 2;
        let d = ((2 * ticks) as u32 % 60) as u8;
        let d = if d > 30 { 60 - d } else { d };
        let mut prims = Vec::new();
        for (i, item) in MENU_ITEMS.iter().enumerate() {
            let colour = if i == self.selected {
                [TITLE[0].wrapping_add(d), TITLE[1].wrapping_sub(d), TITLE[2]]
            } else {
                GREY
            };
            let y = top + MENU_SPACING * i as i32;
            prims.extend(mess(1, colour, self.scales[i]).draw(font, CENTRE_X, y, item)?);
        }
        Ok(prims)
    }
}

/// The statistics screen after the run and the end menu after it, drawn once per pass from the
/// pass the run ends on.
#[derive(Clone, Debug, Default)]
pub struct EndScreens {
    slides: Option<Slides>,
    menu: Option<EndMenu>,
}

impl EndScreens {
    pub fn pass(&mut self, font: &Font, run: &Run, high_score: i32) -> Result<Vec<Prim>, String> {
        let session = &run.skater.session;
        if !session.ended {
            *self = Self::default();
            return Ok(Vec::new());
        }
        if session.menu {
            let menu = self.menu.get_or_insert_with(EndMenu::new);
            menu.step();
            return menu.draw(font, run.ticks);
        }
        let slides = self.slides.get_or_insert_with(Slides::new);
        slides.step();
        let panel = &run.skater.panel;
        if !panel.history.is_empty() {
            return Err(format!(
                "[hud] statistics after scored tricks unsupported tricks={} landed and best combo are not counted",
                panel.history.len()
            ));
        }
        let mut prims =
            mess(1, PROMPT, 0xc0).draw(font, CENTRE_X, 222, "Press ENTER to continue")?;
        prims.extend(mess(1, TITLE, 0x100).draw(font, CENTRE_X, 40, "Statistics")?);
        let values = [panel.score, high_score, panel.landed];
        for ((index, y), label) in ROWS
            .iter()
            .zip(["Total Score", "High Score", "Tricks Landed"])
        {
            prims.extend(mess(0, GREY, 0x100).draw(font, slides.labels[*index], *y, label)?);
        }
        for ((index, y), value) in ROWS.iter().zip(values) {
            prims.extend(mess(2, GREY, 0x100).draw(
                font,
                slides.values[*index],
                *y,
                &value.to_string(),
            )?);
        }
        if slides.labels[0] != LABEL_STOP {
            return Ok(prims);
        }
        match &panel.best {
            None => {
                prims.extend(mess(1, GREY, 0xdc).draw(font, CENTRE_X, 115, "Best Combo")?);
                prims.extend(mess(1, GREY, 0x100).draw(font, CENTRE_X, 140, "None")?);
            }
            Some(best) => {
                let plural = if best.tricks == 1 { "Trick" } else { "Tricks" };
                let line = format!(
                    "Best Combo - {} {plural} - {} Points",
                    best.tricks, best.points
                );
                prims.extend(mess(1, GREY, 0xdc).draw(font, CENTRE_X, 115, &line)?);
                for (k, text) in best.lines.iter().enumerate() {
                    prims.extend(mess(1, GREY, 0xbc).draw(
                        font,
                        CENTRE_X,
                        138 + 14 * k as i32,
                        text,
                    )?);
                }
            }
        }
        Ok(prims)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontId {
    Time,
    Score,
    Mess,
    Tricks,
}

/// Panel images of PANEL.PRE.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageId {
    Score,
    Special,
    Switch,
    Blood,
}

impl ImageId {
    pub const ALL: [(Self, &'static str); 4] = [
        (Self::Score, "scorep.bmp"),
        (Self::Special, "spec_bar.bmp"),
        (Self::Switch, "s2switch.bmp"),
        (Self::Blood, "s2blood.bmp"),
    ];
}

/// One overlay primitive in submission order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Overlay {
    Glyph(FontId, Prim),
    /// An image at its top left corner, raw images skip the colour modulation.
    Image {
        image: ImageId,
        at: [i32; 2],
        raw: bool,
    },
    /// A horizontal gradient strip, left colour at x and right colour len pixels along.
    Bar {
        at: [i32; 2],
        len: i32,
        height: i32,
        left: [u8; 3],
        right: [u8; 3],
    },
}

const SCORE_AT: [i32; 2] = [25, 45];
const STANCE_AT: [i32; 2] = [430, 5];

pub const SCORE: Text = Text {
    hjust: 2,
    vjust: 3,
    spacing: 4,
    colour: [0x80, 0x80, 0x80],
    shadow: Some([2, 2]),
    fixed_pitch: false,
    scale: 0x1000,
    map: CharMap::Digits,
};

/// The special meter: the fill while charging, a flashing bar while the special is on.
fn meter(skater: &Skater, vb: i32) -> Option<Overlay> {
    let (fill, special) = (skater.special.fill, skater.special.meter);
    let at = [SCORE_AT[0] - 1, SCORE_AT[1] + 10];
    if special >= 0x1000 {
        let c = ((vb as u32 % 40) * 6) as u8;
        let len = (special >> 12) * 120 / 750;
        return Some(Overlay::Bar {
            at,
            len,
            height: 3,
            left: [c, c, 0x32],
            right: [c, c, 0x32],
        });
    }
    (fill >= 0x1000).then(|| Overlay::Bar {
        at,
        len: (fill >> 12) * 120 / 3000,
        height: 3,
        left: [0, 0x96, 0x32],
        right: [0, 0x96, 0],
    })
}

/// The single player panels in submission order: stance icon, the pending points and trick lines,
/// clock, then the score panel with its meter, labels and digits. Nothing is drawn while the end
/// screens pause the game.
pub fn panels(
    [time_font, score_font]: [&Font; 2],
    skater: &Skater,
    tricks: &[Prim],
    time: i32,
    vb: i32,
) -> Result<Vec<Overlay>, String> {
    let mut out = Vec::new();
    if skater.session.ended {
        return Ok(out);
    }
    if skater.switch_bonus {
        out.push(Overlay::Image {
            image: ImageId::Switch,
            at: STANCE_AT,
            raw: true,
        });
    }
    out.extend(tricks.iter().map(|&p| Overlay::Glyph(FontId::Tricks, p)));
    out.extend(
        clock(time_font, &skater.session, time, vb)?
            .into_iter()
            .map(|p| Overlay::Glyph(FontId::Time, p)),
    );
    out.extend(meter(skater, vb));
    let [x, y] = SCORE_AT;
    out.push(Overlay::Image {
        image: ImageId::Score,
        at: [x, y - 20],
        raw: false,
    });
    out.push(Overlay::Image {
        image: ImageId::Special,
        at: [x - 4, y],
        raw: false,
    });
    let digits = SCORE.draw(score_font, x + 125, y - 4, &skater.panel.score.to_string())?;
    out.extend(digits.into_iter().map(|p| Overlay::Glyph(FontId::Score, p)));
    Ok(out)
}
