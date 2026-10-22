use crate::{font::Font, raster::ScreenVertex};

/// Text positions are in a 512 by 240 screen, 20.12 fixed point while laying out.
const VIRTUAL_WIDTH_SHIFT: u32 = 9;
const VIRTUAL_HEIGHT: u32 = 240;
const MAX_CHARS: usize = 64;
const PUNCTUATION: &[u8] = b"?!:.,-%+'#/$_";
const SHADOW_COLOUR: [u8; 3] = [0xff; 3];

/// Which characters a font's glyphs stand for.
#[derive(Clone, Copy, Debug)]
pub enum CharMap {
    /// Letters either case, digits, then punctuation.
    Letters,
    /// Digits and a colon.
    Digits,
}

impl CharMap {
    /// The glyph a character draws, none for a space or a colour code. Characters the font has no
    /// glyph for draw the blank glyph after the last one.
    pub fn glyph(self, c: u8, count: usize) -> Option<u8> {
        let mapped = match (self, c) {
            (_, b' ') => return None,
            (CharMap::Letters, b'a'..=b'z') => Some(c - b'a'),
            (CharMap::Letters, b'A'..=b'Z') => Some(c - b'A'),
            (CharMap::Letters, b'0'..=b'9') => Some(c - b'0' + 26),
            (CharMap::Letters, _) => PUNCTUATION
                .iter()
                .position(|&p| p == c)
                .map(|i| 36 + i as u8)
                .filter(|&g| (g as usize) < count),
            (CharMap::Digits, b'0'..=b'9') => Some(c - b'0'),
            (CharMap::Digits, b':') => Some(10),
            (CharMap::Digits, _) => None,
        };
        match mapped {
            Some(g) => Some(g),
            None if colour_code(c).is_some() => None,
            None => Some(count as u8),
        }
    }
}

enum Code {
    Restore,
    Colour([u8; 3]),
    Nothing,
}

fn colour_code(c: u8) -> Option<Code> {
    Some(match c {
        b']' | b'}' | b'>' => Code::Restore,
        b'[' => Code::Colour([0, 0x80, 0xff]),
        b'{' => Code::Colour([0xff, 0x80, 0]),
        b'<' => Code::Colour([0x40, 0x40, 0x40]),
        b'|' => Code::Colour([0x80, 0x40, 0x40]),
        b'~' => Code::Colour([0x40, 0x80, 0x40]),
        b'^' => Code::Nothing,
        _ => return None,
    })
}

/// A text object: the style a string is laid out and coloured with.
#[derive(Clone, Copy, Debug)]
pub struct Text {
    /// 0 starts at x, 1 centres on it, 2 ends at it.
    pub hjust: u8,
    /// 0 top at y, 1 centred, 2 bottom at y, 3 baseline at y.
    pub vjust: u8,
    pub spacing: i32,
    /// PS1 colour, 0x80 is the unlit level.
    pub colour: [u8; 3],
    /// Each glyph is followed by a darkening copy this far off.
    pub shadow: Option<[i32; 2]>,
    pub fixed_pitch: bool,
    /// 20.12 fixed point.
    pub scale: i32,
    pub map: CharMap,
}

/// One glyph primitive as submitted, its box in the virtual screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Prim {
    pub glyph: u8,
    pub shadow: bool,
    pub colour: [u8; 3],
    pub rect: [i32; 4],
    /// An image the text's effect submits right after this glyph.
    pub after: Option<Turned>,
}

/// A rotated image on the virtual screen, corners in the original's primitive order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Turned {
    pub corners: [[i32; 2]; 4],
    /// Far texel corner.
    pub uv: [u8; 2],
    pub grey: u8,
    pub semi: bool,
    /// Draw area x, y, width, height the image is clipped to while it is drawn.
    pub area: [i32; 4],
}

impl Turned {
    pub fn quad(&self, screen: [u32; 2], gamma: &[u8; 256]) -> ScreenQuad {
        let [p0, p1, p2, p3] = self
            .corners
            .map(|[x, y]| [screen_x(x, screen), screen_y(y, screen)]);
        let [u, v] = self.uv;
        ScreenQuad {
            corners: [p0, p1, p3, p2],
            uv: [[0, 0], [u, 0], [u, v], [0, v]],
            diffuse: 0xff00_0000 | (u32::from(gamma[usize::from(self.grey)]) * 0x0001_0101),
        }
    }
}

/// What a text effect does to each glyph of a string while it is laid out and drawn.
pub trait Effect {
    /// Moves a glyph's top, 20.12 fixed point from the pen, once per glyph in string order.
    fn offset(&mut self, _y: &mut i32) {}
    /// The colour every glyph takes instead of the text's own.
    fn colour(&mut self) -> Option<[u8; 3]> {
        None
    }
    /// Runs before the bottom trick line is drawn, where the panel colours the blood image.
    fn before_bottom_line(&mut self) {}
    /// An image to submit after a glyph, given its x, its top before the offset and after it,
    /// all 20.12 on the screen, and the glyph's byte index in the string.
    fn post(&mut self, _x: i32, _top: i32, _moved: i32, _index: usize) -> Option<Turned> {
        None
    }
}

pub struct Plain;

impl Effect for Plain {}

struct Placed {
    glyph: Option<u8>,
    code: Option<Code>,
    x: i32,
    y: i32,
    /// The glyph's top before the effect moved it.
    top: i32,
}

impl Text {
    /// The primitives in submission order. The overlay slot draws the newest first, so a string
    /// shows its last shadow under everything else.
    pub fn draw(&self, font: &Font, x: i32, y: i32, s: &str) -> Result<Vec<Prim>, String> {
        self.draw_with(font, x, y, s, &mut Plain)
    }

    pub fn draw_with(
        &self,
        font: &Font,
        x: i32,
        y: i32,
        s: &str,
        effect: &mut dyn Effect,
    ) -> Result<Vec<Prim>, String> {
        if self.scale == 0 && !self.fixed_pitch {
            return Ok(Vec::new());
        }
        if s.len() > MAX_CHARS {
            return Err(format!(
                "[text] chars={} max={MAX_CHARS} text={s:?}",
                s.len()
            ));
        }
        let count = font.glyphs.len() - 1;
        let scale = self.scale;
        let first = &font.glyphs[0];
        let cell = |c: u8| {
            let width = if matches!(self.map, CharMap::Digits) && c == b':' {
                font.glyphs[10].width
            } else {
                first.width
            };
            ((i32::from(width) + self.spacing) * scale) >> 12
        };
        let (mut pen, mut ascent, mut descent) = (0, 0, 0);
        let mut placed = Vec::with_capacity(s.len());
        for c in s.bytes() {
            let glyph = self.map.glyph(c, count);
            let code = colour_code(c);
            let Some(g) = glyph else {
                if code.is_none() {
                    let w = i32::from(first.width) * scale;
                    pen += w - (w as f64 * 0.25) as i32;
                }
                placed.push(Placed {
                    glyph,
                    code,
                    x: 0,
                    y: 0,
                    top: 0,
                });
                continue;
            };
            let glyph_box = &font.glyphs[g as usize];
            let (w, h, base) = (
                i32::from(glyph_box.width),
                i32::from(glyph_box.height),
                i32::from(glyph_box.base),
            );
            let mut gx = pen;
            ascent = ascent.max(base * scale);
            descent = descent.max((h - base) * scale);
            if self.fixed_pitch {
                gx += (cell(c) << 12) - w * scale;
                pen += cell(c) << 12;
            } else {
                pen += (w + self.spacing) * scale;
            }
            let top = -base * scale;
            let mut gy = top;
            effect.offset(&mut gy);
            placed.push(Placed {
                glyph,
                code,
                x: gx,
                y: gy,
                top,
            });
        }
        let origin_x = match self.hjust {
            0 => x << 12,
            1 => (x << 12) - (pen >> 1),
            2 => (x << 12) - pen,
            j => return Err(format!("[text] hjust={j} unsupported")),
        };
        let origin_y = match self.vjust {
            0 => (y << 12) + ascent,
            1 => (y << 12) + ((ascent + descent) >> 1) - descent,
            2 => (y << 12) - descent,
            3 => y << 12,
            j => return Err(format!("[text] vjust={j} unsupported")),
        };
        let (mut colour, mut saved) = (self.colour, self.colour);
        let mut prims = Vec::with_capacity(2 * placed.len());
        for (index, p) in placed.iter().enumerate() {
            match p.code {
                Some(Code::Restore) => colour = saved,
                Some(Code::Colour(c)) => (saved, colour) = (colour, c),
                Some(Code::Nothing) | None => {}
            }
            let Some(g) = p.glyph else { continue };
            if let Some(c) = effect.colour() {
                colour = c;
            }
            let glyph = &font.glyphs[g as usize];
            let rect = |dx: i32, dy: i32| {
                let x0 = (p.x + dx * scale + origin_x) >> 12;
                let y0 = (p.y + dy * scale + origin_y) >> 12;
                [
                    x0,
                    y0,
                    x0 + ((i32::from(glyph.width) * scale) >> 12),
                    y0 + ((i32::from(glyph.height) * scale) >> 12),
                ]
            };
            prims.push(Prim {
                glyph: g,
                shadow: false,
                colour,
                rect: rect(0, 0),
                after: None,
            });
            if let Some([dx, dy]) = self.shadow {
                prims.push(Prim {
                    glyph: g,
                    shadow: true,
                    colour: SHADOW_COLOUR,
                    rect: rect(dx, dy),
                    after: None,
                });
            }
            let after = effect.post(p.x + origin_x, p.top + origin_y, p.y + origin_y, index);
            if let Some(last) = prims.last_mut() {
                last.after = after;
            }
        }
        Ok(prims)
    }
}

impl Text {
    /// Width a string takes at the text's scale: a space is glyph 0 wide, colour codes take none.
    pub fn width(&self, font: &Font, s: &str) -> i32 {
        let count = font.glyphs.len() - 1;
        let sum: i32 = s
            .bytes()
            .map(|c| match self.map.glyph(c, count) {
                Some(g) => i32::from(font.glyphs[g as usize].width) + self.spacing,
                None if colour_code(c).is_none() => i32::from(font.glyphs[0].width),
                None => 0,
            })
            .sum();
        (sum * self.scale) >> 12
    }

    /// Deepest descent plus tallest ascent of a string's glyphs at the text's scale.
    pub fn height(&self, font: &Font, s: &str) -> i32 {
        let count = font.glyphs.len() - 1;
        let (mut below, mut above) = (0, 0);
        for g in s.bytes().filter_map(|c| self.map.glyph(c, count)) {
            let glyph = &font.glyphs[g as usize];
            below = below.max(i32::from(glyph.height) - i32::from(glyph.base));
            above = above.max(i32::from(glyph.base));
        }
        ((below * self.scale) >> 12) + ((above * self.scale) >> 12)
    }
}

/// A glyph primitive as the PC renderer sends it: four pre-transformed corners on the real
/// screen, the glyph's texel corners and the lit diffuse colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenQuad {
    pub corners: [[i32; 2]; 4],
    pub uv: [[u8; 2]; 4],
    pub diffuse: u32,
}

impl ScreenQuad {
    pub fn new(prim: &Prim, font: &Font, screen: [u32; 2], gamma: &[u8; 256]) -> Self {
        let glyph = &font.glyphs[prim.glyph as usize];
        let [r, g, b] = prim.colour.map(|c| u32::from(gamma[c as usize]));
        // the shadow is subtractive, its diffuse alpha is zero
        let alpha = if prim.shadow { 0 } else { 0xff };
        let uv = [glyph.width - 1, glyph.height - 1];
        Self::rect(prim.rect, uv, alpha << 24 | r << 16 | g << 8 | b, screen)
    }

    /// A box of the virtual screen on the real one, with its far texel corner.
    pub fn rect(
        [x0, y0, x1, y1]: [i32; 4],
        [u, v]: [u8; 2],
        diffuse: u32,
        screen: [u32; 2],
    ) -> Self {
        let sx = |x: i32| screen_x(x, screen);
        let sy = |y: i32| screen_y(y, screen);
        Self {
            corners: [
                [sx(x0), sy(y0)],
                [sx(x1), sy(y0)],
                [sx(x1), sy(y1)],
                [sx(x0), sy(y1)],
            ],
            uv: [[0, 0], [u, 0], [u, v], [0, v]],
            diffuse,
        }
    }
}

/// The device viewport a draw area becomes, none when it would reach past the target and the
/// device keeps its viewport.
pub fn draw_area_viewport([x, y, w, h]: [i32; 4], screen: [u32; 2]) -> Option<[usize; 4]> {
    let [sw, sh] = screen;
    // the original works on unsigned products, wraps y into the 256 line buffer and clamps
    let across = |v: i32| ((v as u32).wrapping_mul(sw) >> VIRTUAL_WIDTH_SHIFT).min(sw);
    let down = |v: i32| ((v as u32).wrapping_mul(sh) / VIRTUAL_HEIGHT).min(sh);
    let (x, w, y, h) = (across(x), across(w), down(y % 256), down(h));
    (x + w <= sw && y + h <= sh).then_some([x, y, w, h].map(|v| v as usize))
}

// the products wrap and the y divide is unsigned like the original's
fn screen_x(x: i32, screen: [u32; 2]) -> i32 {
    (x.wrapping_mul(screen[0] as i32) >> VIRTUAL_WIDTH_SHIFT) as i16 as i32
}

fn screen_y(y: i32, screen: [u32; 2]) -> i32 {
    (y.wrapping_mul(screen[1] as i32) as u32 / VIRTUAL_HEIGHT) as u16 as i16 as i32
}

impl ScreenQuad {
    /// The vertices handed to the device: texel corners over the surface size less one.
    pub fn vertices(&self, [width, height]: [usize; 2]) -> [ScreenVertex; 4] {
        let scale = |size: usize| {
            if size > 1 {
                1.0 / (size - 1) as f32
            } else {
                1.0 / size as f32
            }
        };
        std::array::from_fn(|i| ScreenVertex {
            x: self.corners[i][0] as f32,
            y: self.corners[i][1] as f32,
            z: 1.0,
            rhw: 1.0,
            diffuse: self.diffuse,
            uv: [
                f32::from(self.uv[i][0]) * scale(width),
                f32::from(self.uv[i][1]) * scale(height),
            ],
        })
    }
}

/// Colour levels through the display gamma, setting -15..15 in tenths around 2.0.
pub fn gamma_table(setting: i32) -> [u8; 256] {
    let gamma = f64::from(setting) * 0.1 + 2.0;
    let step = f64::from(1.0f32 / 255.0);
    std::array::from_fn(|i| ((i as f64 * step).powf(1.0 / gamma) * 255.0 + 0.5).min(255.0) as u8)
}
