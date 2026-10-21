use crate::{
    anim::AnimationTable,
    camera::{Camera, Moment, Tripod},
    collision::CollisionWorld,
    font::Font,
    ground::{Level, Outcome, Pad, Rules, Run, TONY_HAWK_STATS},
    hud::{self, EndScreens, FontId, ImageId, Overlay},
    input::KeyTable,
    materials::{TONY_HAWK_LOGOS, apply_logos, resolve_textures},
    pkr::Archive,
    pre::Pre,
    psx::Scene,
    rail::Rails,
    raster::{BLEND_ZERO, DrawState, Raster, Texture},
    render::{self, HANGAR_VIEWPORT, TextureInfo, View},
    script::TrickScripts,
    sprite::Sprite,
    text::{ScreenQuad, gamma_table},
    trig::Triggers,
};

/// The loader picks the first sponsor at random, this is the pick of the reference runs.
const LOGO_START: usize = 2;
/// Clear colour of the Hangar frame after gamma.
const CLEAR: u32 = 0x0020_2d32;
const HIGH_SCORE: i32 = 40000;

pub const TONY: Rules = Rules {
    stats: TONY_HAWK_STATS,
    handicap: None,
    auto_kick: true,
    deck_turning: 1,
};

/// Everything a Hangar single session with Tony Hawk reads from the original data.
pub struct Hangar {
    pub scene: Scene,
    world: CollisionWorld,
    table: AnimationTable,
    triggers: Triggers,
    scripts: TrickScripts,
    rails: Rails,
    keys: KeyTable,
    textures: Vec<Texture>,
    infos: Vec<TextureInfo>,
    time_font: Font,
    mess_font: Font,
    time_glyphs: Vec<Texture>,
    mess_glyphs: Vec<Texture>,
    score_font: Font,
    score_glyphs: Vec<Texture>,
    sprites: Vec<(ImageId, Sprite)>,
}

fn glyphs(font: &Font) -> Vec<Texture> {
    (0..font.glyphs.len())
        .map(|g| font.texture(g as u8))
        .collect()
}

fn load_scene(archive: &Archive, path: &str) -> Result<Scene, String> {
    Scene::parse(archive.read(path).map_err(|e| e.to_string())?)
        .map_err(|e| format!("entry={path} {e}"))
}

impl Hangar {
    pub fn load(archive: &Archive) -> Result<Self, String> {
        let read = |path: &str| archive.read(path).map_err(|e| format!("entry={path} {e}"));
        let scene = load_scene(archive, "data/SKHAN.PSX")?;
        let bank = load_scene(archive, "data/SKHAN_L.PSX")?;
        let mut materials = resolve_textures(archive, &scene.texture_checksums, &bank, "Hangar")?;
        apply_logos(&mut materials, archive, &TONY_HAWK_LOGOS, LOGO_START)?;
        let surfaces: Vec<_> = materials.iter().map(|t| t.surface_1555()).collect();
        let triggers = Triggers::parse(read("data/SKHAN_T.TRG")?)?;
        let time_font = Font::parse(read("data/TIME.FNT")?)?;
        let mess_font = Font::parse(read("data/INGAME1.FNT")?)?;
        let score_font = Font::parse(read("data/SCORE.FNT")?)?;
        let panel = Pre::parse(read("data/PANEL.PRE")?)?;
        let sprites = ImageId::ALL
            .iter()
            .map(|&(id, name)| {
                let bytes = panel
                    .get(name)
                    .ok_or_else(|| format!("[panel] missing={name}"))?;
                Ok((
                    id,
                    Sprite::from_bmp(bytes).map_err(|e| format!("[panel] image={name} {e}"))?,
                ))
            })
            .collect::<Result<_, String>>()?;
        let scripts = TrickScripts::parse(read("data/TRICKS.BIN")?.to_vec())?;
        Ok(Self {
            world: CollisionWorld::from_scene(&scene)?,
            table: AnimationTable::from_scene(&load_scene(archive, "data/SK2ANIM.PSX")?)?,
            rails: Rails::from_triggers(&triggers, 1)?,
            keys: KeyTable::build(&scripts, 0)?,
            infos: materials
                .iter()
                .zip(&surfaces)
                .map(|(t, s)| TextureInfo {
                    width: t.logical_size[0],
                    height: t.logical_size[1],
                    opaque: s.pixels.iter().all(|p| p & 0x8000 != 0),
                })
                .collect(),
            textures: surfaces
                .into_iter()
                .map(|s| Texture {
                    width: s.width.into(),
                    height: s.height.into(),
                    texels: s.pixels,
                })
                .collect(),
            time_glyphs: glyphs(&time_font),
            mess_glyphs: glyphs(&mess_font),
            time_font,
            mess_font,
            score_glyphs: glyphs(&score_font),
            score_font,
            sprites,
            scene,
            triggers,
            scripts,
        })
    }

    pub fn level(&self) -> Level<'_> {
        Level {
            world: &self.world,
            table: &self.table,
            tricks: &self.scripts,
            triggers: &self.triggers,
            rails: &self.rails,
            keys: &self.keys,
        }
    }
}

/// One single session: the run, the camera following it and the end screens, one frame at a time.
pub struct Session {
    pub run: Run,
    pub camera: Camera,
    screens: EndScreens,
    overlay: Vec<Overlay>,
}

impl Session {
    pub fn start(hangar: &Hangar) -> Result<Self, String> {
        let level = hangar.level();
        let run = Run::start(&level, hangar.triggers.restart_named("Start")?)?;
        let camera = Camera::settled(
            &Tripod::of(&run.skater),
            level.world,
            crate::ground::DESIGN_FRAME.scale,
        )?;
        Ok(Self {
            run,
            camera,
            screens: EndScreens::default(),
            overlay: Vec::new(),
        })
    }

    /// One 30 Hz frame: the skaters, the camera, then the clock and end screens to draw over it.
    pub fn step(&mut self, hangar: &Hangar, pad: Pad) -> Result<(), String> {
        let level = hangar.level();
        // the pass that takes Retry only restarts, the new run's first frame is the next pass
        if self.run.frame(&level, TONY, pad)? == Outcome::Retry {
            *self = Self::start(hangar)?;
            return Ok(());
        }
        let s = &self.run.skater;
        let moment = Moment {
            step: self.run.clock.current().scale,
            time_up: s.session.time_up,
            orbit: s.session.time_up && s.session.wind < 40 && s.idle(),
            paused: s.session.ended,
        };
        let world = self.run.world.as_ref().unwrap_or(level.world);
        self.camera.update(&Tripod::of(s), world, moment)?;
        let time = self.run.clock.current().time;
        self.overlay = hud::panels(&hangar.time_font, &hangar.score_font, s, time, self.run.vb)?;
        let screens = self
            .screens
            .pass(&hangar.mess_font, &self.run, HIGH_SCORE)?;
        self.overlay
            .extend(screens.into_iter().map(|p| Overlay::Glyph(FontId::Mess, p)));
        Ok(())
    }

    /// The frame's draws as the original issues them: the level by depth, then the overlay
    /// newest first.
    pub fn draws<'h>(&self, hangar: &'h Hangar) -> Result<Vec<FrameDraw<'h>>, String> {
        let view = View::new(self.camera.eye, &self.camera.matrix, HANGAR_VIEWPORT);
        let hidden: Vec<usize> = self
            .run
            .world
            .as_ref()
            .map(|w| w.hidden().collect())
            .unwrap_or_default();
        let level = render::Level {
            scene: &hangar.scene,
            textures: &hangar.infos,
            bias: &render::hangar_bias,
            hidden: &|i| hidden.binary_search(&i).is_ok(),
        };
        let gamma = gamma_table(0);
        let polys = view.level(&level, &self.camera.matrix)?;
        let mut draws: Vec<FrameDraw> = view
            .draws(&polys, view.fog([0, 0, 0]), &gamma)
            .into_iter()
            .map(|d| FrameDraw {
                state: d.state(&hangar.infos),
                texture: d.texture.map(|t| &hangar.textures[t as usize]),
                vertices: d.vertices,
            })
            .collect();
        let size = [HANGAR_VIEWPORT.width as u32, HANGAR_VIEWPORT.height as u32];
        for overlay in self.overlay.iter().rev() {
            draws.push(match *overlay {
                Overlay::Glyph(id, prim) => {
                    let (font, glyphs) = match id {
                        FontId::Time => (&hangar.time_font, &hangar.time_glyphs),
                        FontId::Score => (&hangar.score_font, &hangar.score_glyphs),
                        FontId::Mess => (&hangar.mess_font, &hangar.mess_glyphs),
                    };
                    let texture = &glyphs[prim.glyph as usize];
                    let quad = ScreenQuad::new(&prim, font, size, &gamma);
                    FrameDraw {
                        vertices: quad.vertices([texture.width, texture.height]).to_vec(),
                        state: overlay_state(prim.shadow),
                        texture: Some(texture),
                    }
                }
                Overlay::Image {
                    image,
                    at: [x, y],
                    raw,
                } => {
                    let sprite = &hangar
                        .sprites
                        .iter()
                        .find(|(id, _)| *id == image)
                        .ok_or("[panel] image not loaded")?
                        .1;
                    let (w, h) = (i32::from(sprite.width), i32::from(sprite.height));
                    let level = if raw { 0xff } else { u32::from(gamma[0x80]) };
                    let diffuse = 0xff00_0000 | (level * 0x0001_0101);
                    let uv = [(w - 1) as u8, (h - 1) as u8];
                    let quad = ScreenQuad::rect([x, y, x + w, y + h], uv, diffuse, size);
                    FrameDraw {
                        vertices: quad.vertices(sprite.logical).to_vec(),
                        state: overlay_state(false),
                        texture: Some(&sprite.texture),
                    }
                }
                Overlay::Bar {
                    at: [x, y],
                    len,
                    height,
                    left,
                    right,
                } => {
                    let colour = |c: [u8; 3]| {
                        0xff00_0000
                            | c.iter()
                                .fold(0, |a, &k| a << 8 | u32::from(gamma[k as usize]))
                    };
                    let quad = ScreenQuad::rect([x, y, x + len, y + height], [0, 0], 0, size);
                    let mut vertices = quad.vertices([1, 1]);
                    for (v, c) in vertices.iter_mut().zip([left, right, right, left]) {
                        (v.diffuse, v.uv) = (colour(c), [0.0; 2]);
                    }
                    FrameDraw {
                        vertices: vertices.to_vec(),
                        state: DrawState {
                            colour_op: 3,
                            alpha_op: 3,
                            ..overlay_state(false)
                        },
                        texture: None,
                    }
                }
            });
        }
        Ok(draws)
    }

    pub fn compose(&self, hangar: &Hangar) -> Result<Raster, String> {
        let mut raster = Raster::new(
            HANGAR_VIEWPORT.width as usize,
            HANGAR_VIEWPORT.height as usize,
        );
        raster.clear(CLEAR, 0.0);
        for d in self.draws(hangar)? {
            raster.fan(&d.vertices, &d.state, d.texture)?;
        }
        Ok(raster)
    }
}

/// One draw call of a composed frame.
pub struct FrameDraw<'h> {
    pub vertices: Vec<crate::raster::ScreenVertex>,
    pub state: DrawState,
    pub texture: Option<&'h Texture>,
}

/// Overlay glyphs blend over the frame, their shadows darken it, neither touches depth.
fn overlay_state(shadow: bool) -> DrawState {
    let (src, dst) = if shadow { (BLEND_ZERO, 4) } else { (5, 6) };
    DrawState {
        blend: true,
        src,
        dst,
        depth_test: false,
        depth_write: false,
        depth_func: 7,
        colour_op: 4,
        alpha_op: 4,
        wrap: false,
        gouraud: true,
    }
}
