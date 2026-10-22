use crate::{
    anim::AnimationTable,
    collision::CollisionWorld,
    figure::{
        Figure, HANGAR_LIGHT, HANGAR_LIGHT_SCALE, Placement, Shared, object_matrix, shadow_colour,
        shadow_matrix, tilted_basis,
    },
    fixes::Fixes,
    font::Font,
    ground::{Level, Outcome, Pad, PropModels, Rules, Run, TONY_HAWK_STATS},
    hud::{self, EndScreens, FontId, ImageId, Overlay},
    input::KeyTable,
    materials::{TONY_HAWK_LOGOS, apply_logos, resolve_textures},
    pkr::Archive,
    pose::Rig,
    pre::Pre,
    psx::Scene,
    rail::Rails,
    raster::{BLEND_ONE, BLEND_ZERO, DrawState, Raster, Texture},
    render::{self, HANGAR_VIEWPORT, Placed, Poly, TextureInfo, View},
    script::TrickScripts,
    sprite::Sprite,
    text::{Prim, ScreenQuad, draw_area_viewport, gamma_table},
    tricktext::TrickText,
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
    trick_font: Font,
    trick_glyphs: Vec<Texture>,
    sprites: Vec<(ImageId, Sprite)>,
    figure: Figure,
    /// Index of the skater's first texture, after the level's.
    figure_textures: usize,
    /// The level's object bank, the models props draw, and where its textures start.
    objects: Scene,
    prop_models: PropModels,
    object_infos: Vec<TextureInfo>,
    object_textures: usize,
}

fn texture_infos(materials: &[crate::materials::Texture]) -> Vec<TextureInfo> {
    materials
        .iter()
        .map(|t| {
            let s = t.surface_1555();
            TextureInfo {
                width: t.logical_size[0],
                height: t.logical_size[1],
                opaque: s.pixels.iter().all(|p| p & 0x8000 != 0),
            }
        })
        .collect()
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
        // the level mounts the panel archive first, so its fonts win over the loose ones
        let panel = Pre::parse(read("data/PANEL.PRE")?)?;
        let font = |name: &str| {
            Font::parse(
                panel
                    .get(name)
                    .ok_or_else(|| format!("[panel] missing={name}"))?,
            )
            .map_err(|e| format!("[panel] font={name} {e}"))
        };
        let time_font = font("time.fnt")?;
        let mess_font = font("ingame1.fnt")?;
        let score_font = font("score.fnt")?;
        let trick_font = font("s2tricks.fnt")?;
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
        let (figure, figure_surfaces) = load_figure(archive)?;
        let objects = load_scene(archive, "data/SKHAN_O.PSX")?;
        let object_materials =
            resolve_textures(archive, &objects.texture_checksums, &objects, "Hangar")?;
        let object_infos = texture_infos(&object_materials);
        let figure_textures = surfaces.len();
        let object_textures = figure_textures + figure_surfaces.len();
        let textures = surfaces
            .into_iter()
            .chain(figure_surfaces)
            .chain(object_materials.iter().map(|t| t.surface_1555()))
            .map(|s| Texture {
                width: s.width.into(),
                height: s.height.into(),
                texels: s.pixels,
            })
            .collect();
        let mut infos = texture_infos(&materials);
        infos.extend(figure.textures.iter().copied());
        infos.extend(object_infos.iter().copied());
        Ok(Self {
            world: CollisionWorld::from_scene(&scene)?,
            table: AnimationTable::from_scene(&load_scene(archive, "data/SK2ANIM.PSX")?)?,
            rails: Rails::from_triggers(&triggers, 1)?,
            keys: KeyTable::build(&scripts, 0)?,
            infos,
            textures,
            figure,
            figure_textures,
            prop_models: PropModels::from_scene(&objects),
            objects,
            object_infos,
            object_textures,
            time_glyphs: glyphs(&time_font),
            mess_glyphs: glyphs(&mess_font),
            time_font,
            mess_font,
            score_glyphs: glyphs(&score_font),
            score_font,
            trick_glyphs: glyphs(&trick_font),
            trick_font,
            sprites,
            scene,
            triggers,
            scripts,
        })
    }

    /// The trick line font, which a run without the viewer needs for the panel's random draws.
    pub fn trick_font(&self) -> &Font {
        &self.trick_font
    }

    pub fn level(&self) -> Level<'_> {
        Level {
            world: &self.world,
            table: &self.table,
            tricks: &self.scripts,
            triggers: &self.triggers,
            rails: &self.rails,
            keys: &self.keys,
            props: &self.prop_models,
        }
    }
}

/// The skater drawn from Tony Hawk's model with the shared animation bank.
fn load_figure(archive: &Archive) -> Result<(Figure, Vec<crate::materials::Surface1555>), String> {
    let read = |path: &str| archive.read(path).map_err(|e| format!("entry={path} {e}"));
    let text = |path: &str| -> Result<String, String> {
        String::from_utf8(read(path)?.to_vec()).map_err(|e| format!("entry={path} {e}"))
    };
    let scene = load_scene(archive, "data/HAWK2.PSX")?;
    let rig = Rig::from_scene(&load_scene(archive, "data/SK2ANIM.PSX")?)?;
    let materials = resolve_textures(archive, &scene.texture_checksums, &scene, "Hangar")?;
    let surfaces: Vec<_> = materials.iter().map(|t| t.surface_1555()).collect();
    let infos = materials
        .iter()
        .zip(&surfaces)
        .map(|(t, s)| TextureInfo {
            width: t.logical_size[0],
            height: t.logical_size[1],
            opaque: s.pixels.iter().all(|p| p & 0x8000 != 0),
        })
        .collect();
    let figure = Figure::new(
        scene,
        rig,
        &text("data/HAWK2.PSH")?,
        &text("data/SK2ANIM.PSH")?,
        infos,
    )?;
    Ok((figure, surfaces))
}

/// One single session: the run, the camera following it and the end screens, one frame at a time.
pub struct Session {
    pub run: Run,
    pub fixes: Fixes,
    screens: EndScreens,
    text: TrickText,
    overlay: Vec<Overlay>,
    shared: Shared,
    clip: Option<(u16, Vec<Vec<[i16; 6]>>)>,
    figure: Vec<Poly>,
}

impl Session {
    pub fn start(hangar: &Hangar) -> Result<Self, String> {
        let level = hangar.level();
        let run = Run::start(&level, hangar.triggers.restart_named("Start")?)?;
        Ok(Self {
            run,
            fixes: Fixes::default(),
            screens: EndScreens::default(),
            text: TrickText::default(),
            overlay: Vec::new(),
            shared: Shared::default(),
            clip: None,
            figure: Vec::new(),
        })
    }

    /// One 30 Hz frame: the skaters, the camera, then the clock and end screens to draw over it.
    pub fn step(&mut self, hangar: &Hangar, pad: Pad) -> Result<(), String> {
        let level = hangar.level();
        // the pass that takes Retry only restarts, the new run's first frame is the next pass
        if self.run.frame(&level, TONY, pad)? == Outcome::Retry {
            let fixes = self.fixes;
            *self = Self::start(hangar)?;
            self.fixes = fixes;
            return Ok(());
        }
        let tricks = self
            .text
            .pass(&hangar.trick_font, &mut self.run.skater, self.fixes)?;
        self.figure = self.figure_polys(hangar)?;
        let s = &self.run.skater;
        let time = self.run.clock.current().time;
        let fonts = [&hangar.time_font, &hangar.score_font];
        self.overlay = hud::panels(fonts, s, &tricks, time, self.run.vb)?;
        let screens = self.screens.pass(
            [&hangar.mess_font, &hangar.trick_font],
            &self.run,
            HIGH_SCORE,
        )?;
        self.overlay
            .extend(screens.into_iter().map(|p| Overlay::Glyph(FontId::Mess, p)));
        Ok(())
    }

    /// The frame's draws as the original issues them: the level by depth, then the overlay
    /// newest first.
    pub fn draws<'h>(&self, hangar: &'h Hangar) -> Result<Vec<FrameDraw<'h>>, String> {
        let view = View::new(self.run.camera.eye, &self.run.camera.matrix, HANGAR_VIEWPORT);
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
        let mut polys = view.level(&level, &self.run.camera.matrix)?;
        polys.extend(self.props(hangar, &view)?);
        polys.extend(self.figure.iter().cloned());
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
        let sprite = |image: ImageId| {
            hangar
                .sprites
                .iter()
                .find(|(id, _)| *id == image)
                .map(|(_, s)| s)
                .ok_or("[panel] image not loaded")
        };
        for overlay in self.overlay.iter().rev() {
            // drawn newest first, so an image submitted after its glyph comes before it
            if let Overlay::Glyph(_, Prim { after: Some(t), .. }) = overlay {
                let blood = sprite(ImageId::Blood)?;
                let state = DrawState {
                    viewport: draw_area_viewport(t.area, size),
                    ..overlay_state(false)
                };
                draws.push(FrameDraw {
                    vertices: t.quad(size, &gamma).vertices(blood.logical).to_vec(),
                    state: if t.semi {
                        DrawState {
                            dst: BLEND_ONE,
                            ..state
                        }
                    } else {
                        state
                    },
                    texture: Some(&blood.texture),
                });
            }
            draws.push(match *overlay {
                Overlay::Glyph(id, prim) => {
                    let (font, glyphs) = match id {
                        FontId::Time => (&hangar.time_font, &hangar.time_glyphs),
                        FontId::Score => (&hangar.score_font, &hangar.score_glyphs),
                        FontId::Mess => (&hangar.mess_font, &hangar.mess_glyphs),
                        FontId::Tricks => (&hangar.trick_font, &hangar.trick_glyphs),
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
                    let sprite = sprite(image)?;
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

    /// The skater's polygons for the frame, its pose from the current anim frame.
    fn figure_polys(&mut self, hangar: &Hangar) -> Result<Vec<Poly>, String> {
        let s = &self.run.skater;
        let anim = s.anim.id;
        if self.clip.as_ref().is_none_or(|(id, _)| *id != anim) {
            self.clip = Some((anim, hangar.figure.rig.decode(usize::from(anim))?));
        }
        let frames = &self.clip.as_ref().expect("clip decoded above").1;
        let frame = frames.get(s.anim.frame as usize).ok_or_else(|| {
            format!(
                "[figure] anim={anim} frame={} frames={}",
                s.anim.frame,
                frames.len()
            )
        })?;
        let pose = hangar.figure.rig.pose(frame);
        let view = View::new(self.run.camera.eye, &self.run.camera.matrix, HANGAR_VIEWPORT);
        let basis = tilted_basis(s.basis_matrix(), s.balance.angle);
        let place = Placement {
            pose: &pose,
            orientation: object_matrix(basis),
            position: s.position,
            light: HANGAR_LIGHT.scaled(HANGAR_LIGHT_SCALE, s.brightness),
            mirrored: s.switched,
            hidden: if s.board_hidden {
                hangar.figure.board
            } else {
                0
            },
            shadow: None,
        };
        let mut polys = view.figure(&hangar.figure, &place, &mut self.shared)?;
        if s.shadow {
            let shadow = Placement {
                orientation: shadow_matrix(basis, s.shadow_normal),
                position: s.shadow_point,
                shadow: Some(shadow_colour(s.brightness)),
                ..place
            };
            polys.extend(view.figure(&hangar.figure, &shadow, &mut self.shared)?);
        }
        for p in &mut polys {
            p.texture = p.texture.map(|t| t + hangar.figure_textures as u32);
        }
        Ok(polys)
    }

    /// The props in their draw list order, from the object bank.
    fn props(&self, hangar: &Hangar, view: &View) -> Result<Vec<Poly>, String> {
        // a prop without a model stays hidden
        let placed = self
            .run
            .props
            .iter()
            .enumerate()
            .filter(|(_, p)| p.frozen_at.is_none() && !p.dead && !p.hidden)
            .filter_map(|(index, p)| p.model.map(|m| (index, p, m)))
            .map(|(index, p, checksum)| {
                let model = hangar
                    .objects
                    .models
                    .iter()
                    .position(|m| m.checksum == checksum)
                    .ok_or_else(|| {
                        format!(
                            "[props] node={} model={checksum:08x} not in the object bank",
                            p.node
                        )
                    })?;
                Ok(Placed {
                    index,
                    model,
                    position: p.position,
                    angles: p.angles,
                    open_far: false,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let bank = render::Level {
            scene: &hangar.objects,
            textures: &hangar.object_infos,
            bias: &render::hangar_bias,
            hidden: &|_| false,
        };
        let mut polys = view.items(&bank, &placed, &self.run.camera.matrix)?;
        for p in &mut polys {
            p.texture = p.texture.map(|t| t + hangar.object_textures as u32);
        }
        Ok(polys)
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
        viewport: None,
    }
}
