use bevy::prelude::*;
use sha2::{Digest, Sha256};
use std::{error::Error, path::PathBuf};
use thps2_data::draw::{DrawFace, FaceOverride, PreparedVertex, prepare_face, prepare_vertices};
use thps2_data::materials::{Surface1555, Texture, TextureSource, resolve_textures};
use thps2_data::pkr::Archive;
use thps2_data::psx::Scene;
#[cfg(feature = "viewer")]
pub mod renderer;
pub mod spawn;
pub mod view;
use spawn::{
    SessionCollision, SessionPad, SessionRules, SessionTriggers, SkaterMotion, SkaterPlacement,
    advance_ground, initialize_spawn,
};
use thps2_data::ground::{Pad, Rules, TONY_HAWK_STATS};
use view::{CapturedView, ProjectedInstance, project_hangar};

#[derive(Resource)]
pub struct OriginalData(pub Archive);

#[derive(Resource)]
pub struct OriginalScene(pub Scene);

#[derive(Resource)]
pub struct SkaterScene(pub Scene);

#[derive(Resource)]
pub struct HangarTextures(pub Vec<Texture>);

#[derive(Resource)]
pub struct HangarSurfaces(pub Vec<Surface1555>);

#[derive(Resource)]
pub struct HangarDraws(pub Vec<Vec<DrawFace>>);

#[derive(Resource)]
pub struct HangarVertices(pub Vec<Vec<PreparedVertex>>);

#[derive(Component)]
pub struct SceneInstance {
    pub item_index: usize,
}

#[derive(Resource, Default)]
pub struct SessionAssets {
    pub files: Vec<(&'static str, Vec<u8>)>,
    pub error: Option<String>,
}

pub const HANGAR_FILES: [&str; 6] = [
    "data/SKHAN.PSX",
    "data/SKHAN_T.TRG",
    "data/HAWK.PSX",
    "data/HAWK.PSH",
    "data/SKHAN_L.PSX",
    "data/SK2ANIM.PSX",
];

pub fn load_hangar(
    mut commands: Commands,
    data: Res<OriginalData>,
    mut assets: ResMut<SessionAssets>,
) {
    for path in HANGAR_FILES {
        match data.0.read(path) {
            Ok(bytes) => assets.files.push((path, bytes.to_vec())),
            Err(error) => {
                assets.error = Some(error.to_string());
                return;
            }
        }
    }
    match Scene::parse(&assets.files[2].1) {
        Ok(scene) => commands.insert_resource(SkaterScene(scene)),
        Err(error) => {
            assets.error = Some(format!("entry={} {error}", HANGAR_FILES[2]));
            return;
        }
    }
    match Scene::parse(&assets.files[0].1) {
        Ok(scene) => {
            let textures = Scene::parse(&assets.files[4].1)
                .map_err(|error| format!("entry={} {error}", HANGAR_FILES[4]))
                .and_then(|bank| {
                    resolve_textures(&data.0, &scene.texture_checksums, &bank, "Hangar")
                });
            match textures {
                Ok(textures) => {
                    match prepare_hangar_draws(&scene, &textures) {
                        Ok(draws) => {
                            commands.insert_resource(HangarDraws(draws));
                        }
                        Err(error) => {
                            assets.error = Some(error);
                            return;
                        }
                    }
                    commands.insert_resource(HangarSurfaces(
                        textures.iter().map(Texture::surface_1555).collect(),
                    ));
                    commands.insert_resource(HangarTextures(textures));
                }
                Err(error) => {
                    assets.error = Some(error);
                    return;
                }
            }
            let vertices: Result<Vec<_>, _> = scene
                .models
                .iter()
                .enumerate()
                .map(|(index, model)| {
                    prepare_vertices(&model.vertices)
                        .map_err(|error| format!("model={index} {error}"))
                })
                .collect();
            match vertices {
                Ok(vertices) => {
                    commands.insert_resource(HangarVertices(vertices));
                }
                Err(error) => {
                    assets.error = Some(error);
                    return;
                }
            }
            for item_index in 0..scene.items.len() {
                commands.spawn(SceneInstance { item_index });
            }
            commands.insert_resource(OriginalScene(scene));
        }
        Err(error) => assets.error = Some(format!("entry={} {error}", HANGAR_FILES[0])),
    }
}

fn prepare_hangar_draws(scene: &Scene, textures: &[Texture]) -> Result<Vec<Vec<DrawFace>>, String> {
    scene
        .models
        .iter()
        .enumerate()
        .map(|(model_index, model)| {
            let mut draws = Vec::new();
            for (face_index, face) in model.faces.iter().enumerate() {
                let size = face
                    .texture_index
                    .and_then(|i| textures.get(i as usize))
                    .map(|t| t.logical_size);
                draws.extend(
                    prepare_face(face, FaceOverride::default(), size, true).map_err(|error| {
                        format!("model={model_index} face={face_index} {error}")
                    })?,
                );
            }
            Ok(draws)
        })
        .collect()
}

pub fn inspect(
    root: PathBuf,
    manifest: bool,
    captured: Option<CapturedView>,
    render: bool,
    capture: Option<PathBuf>,
    simulate_frames: u32,
    pad: SessionPad,
) -> Result<(), Box<dyn Error>> {
    let path = root.join("ALL.PKR");
    let bytes =
        std::fs::read(&path).map_err(|error| format!("archive={} {error}", path.display()))?;
    let hash = format!("{:x}", Sha256::digest(&bytes));
    if hash != "7e2ae173577c41598dc644824b52f7eaf9664ea478e177a8703f77c0185207da" {
        return Err(format!("archive={} unsupported sha256={hash}", path.display()).into());
    }
    let archive =
        Archive::parse(bytes).map_err(|error| format!("archive={} {error}", path.display()))?;
    if manifest {
        for entry in archive.entries() {
            let bytes = archive.read(entry.path())?;
            println!(
                "{}\t{}\t{:x}",
                entry.path(),
                bytes.len(),
                Sha256::digest(bytes)
            );
        }
        return Ok(());
    }
    println!(
        "[pkr] archive={} sha256={hash} directories={} files={}",
        path.display(),
        archive.directory_count(),
        archive.entries().len()
    );
    let mut app = App::new();
    if let Some(captured) = captured {
        app.insert_resource(captured);
    }
    app.add_plugins(MinimalPlugins)
        .insert_resource(OriginalData(archive))
        .insert_resource(Time::<Fixed>::from_hz(30.0))
        .insert_resource(SessionRules(Rules {
            stats: TONY_HAWK_STATS,
            handicap: None,
            auto_kick: true,
            deck_turning: 1,
        }))
        .insert_resource(pad)
        .init_resource::<SessionAssets>()
        .add_systems(
            Startup,
            (load_hangar, project_hangar, initialize_spawn).chain(),
        )
        .add_systems(FixedUpdate, advance_ground);
    app.update();
    for _ in 0..simulate_frames {
        app.world_mut().run_schedule(FixedUpdate);
    }
    let assets = app.world().resource::<SessionAssets>();
    if let Some(error) = &assets.error {
        return Err(error.clone().into());
    }
    for (path, bytes) in &assets.files {
        println!(
            "[asset] path={path} bytes={} sha256={:x}",
            bytes.len(),
            Sha256::digest(bytes)
        );
    }
    let mut instances = app.world_mut().query::<&SceneInstance>();
    let scene = &app.world().resource::<OriginalScene>().0;
    let placed_faces: usize = instances
        .iter(app.world())
        .map(|instance| {
            scene.models[scene.items[instance.item_index].model_index as usize]
                .faces
                .len()
        })
        .sum();
    println!(
        "[scene] items={} models={} placed_faces={} textures={} raw_chunks={} render_requested={render} simulation=false",
        scene.items.len(),
        scene.models.len(),
        placed_faces,
        scene.texture_checksums.len(),
        scene.chunks.len()
    );
    let textures = &app.world().resource::<HangarTextures>().0;
    let bitmap_count = textures
        .iter()
        .filter(|t| matches!(t.source, TextureSource::Bitmap { .. }))
        .count();
    println!(
        "[materials] resolved={} bitmap={} indexed={} gpu_upload_requested={render}",
        textures.len(),
        bitmap_count,
        textures.len() - bitmap_count
    );
    let surfaces = &app.world().resource::<HangarSurfaces>().0;
    let transparent = surfaces
        .iter()
        .flat_map(|s| &s.pixels)
        .filter(|&&pixel| pixel == 0)
        .count();
    println!(
        "[surfaces] format=ARGB1555 count={} transparent_pixels={transparent} gpu_upload_requested={render}",
        surfaces.len()
    );
    let vertices = &app.world().resource::<HangarVertices>().0;
    println!(
        "[vertices] records={} screen_offsets={} camera_projection={}",
        vertices.iter().map(Vec::len).sum::<usize>(),
        vertices
            .iter()
            .flatten()
            .filter(|v| matches!(v, PreparedVertex::ScreenOffset { .. }))
            .count(),
        app.world().contains_resource::<CapturedView>()
    );
    let draws = &app.world().resource::<HangarDraws>().0;
    println!(
        "[draws] models={} records={} overrides=identity textured_quad_split=true gpu_submission_requested={render}",
        draws.len(),
        draws.iter().map(Vec::len).sum::<usize>()
    );
    let skater = &app.world().resource::<SkaterScene>().0;
    println!(
        "[skater] models={} embedded_textures={} animation=false",
        skater.models.len(),
        skater.embedded_textures.len()
    );
    if app.world().contains_resource::<CapturedView>() {
        let mut query = app.world_mut().query::<&ProjectedInstance>();
        let instances: Vec<_> = query.iter(app.world()).collect();
        println!(
            "[projection] items={} vertices={} inside={} source_items_only=true mode=normal gpu_submission=false",
            instances.len(),
            instances.iter().map(|i| i.0.len()).sum::<usize>(),
            instances
                .iter()
                .flat_map(|i| &i.0)
                .filter(|v| v.clip == 0)
                .count()
        );
    }
    let triggers = &app.world().resource::<SessionTriggers>().0;
    let collision = &app.world().resource::<SessionCollision>().0;
    println!(
        "[collision] cells={:?} triggers={} restarts={} traversal=zone_walk trigger_execution=false",
        collision.zone.dimensions,
        triggers.nodes.len(),
        triggers.restarts.len()
    );
    let mut skaters = app.world_mut().query::<&SkaterPlacement>();
    for skater in skaters.iter(app.world()) {
        println!(
            "[spawn] selection=explicit_Start node={} restart_q12={:?} standing_q12={:?} source_angles={:?} ground_item={} ground_face={} t_q14={} simulation=fixed_update_30hz",
            skater.restart_node,
            skater.restart_position_fixed,
            skater.position_fixed,
            skater.source_angles,
            skater.ground.item_index,
            skater.ground.face_index,
            skater.ground.t_q14
        );
    }
    let rules = app.world().resource::<SessionRules>().0;
    let mut motions = app.world_mut().query::<&SkaterMotion>();
    for motion in motions.iter(app.world()) {
        println!(
            "[ground] frames={} requested={simulate_frames} scale={} stats={:?} handicap={:?} auto_kick={} state={:?} position_q12={:?} velocity_q12={:?} up_q12={:?} last_ground={:?} halted={:?} pad={:?} turn={} anim={}:{} charge={} ollies={} push_cooldown={} push_target={} brake_mode={}",
            motion.frames,
            motion.run.clock.current().scale,
            rules.stats,
            rules.handicap,
            rules.auto_kick,
            motion.run.skater.state,
            motion.run.skater.position,
            motion.run.skater.velocity,
            motion.run.skater.up,
            motion.last_ground,
            motion.halted,
            pad.held,
            motion.run.skater.turn,
            motion.run.skater.anim.id,
            motion.run.skater.anim.frame,
            motion.run.skater.charge,
            motion.run.skater.ollies,
            motion.run.skater.push_cooldown,
            motion.run.skater.push_target,
            motion.run.skater.brake_mode
        );
    }
    if render {
        #[cfg(feature = "viewer")]
        renderer::run(app.world_mut(), capture)?;
        #[cfg(not(feature = "viewer"))]
        {
            let _ = capture;
            return Err("--render requires cargo feature viewer".into());
        }
    }
    Ok(())
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let mut root = None;
    let mut manifest = false;
    let mut captured = None;
    let mut render = false;
    let mut capture = None;
    let mut simulate_frames = 120;
    let mut pad = Pad::default();
    let mut release_after = None;
    while let Some(arg) = args.next() {
        if arg == "--data-root" {
            root = Some(PathBuf::from(
                args.next().ok_or("--data-root requires a directory")?,
            ));
        } else if arg == "--manifest" {
            manifest = true;
        } else if arg == "--view" {
            let path = PathBuf::from(args.next().ok_or("--view requires a capture file")?);
            let text = std::fs::read_to_string(&path)
                .map_err(|error| format!("view={} {error}", path.display()))?;
            captured = Some(
                CapturedView::parse(&text)
                    .map_err(|error| format!("view={} {error}", path.display()))?,
            );
        } else if arg == "--render" {
            render = true;
        } else if arg == "--capture" {
            capture = Some(PathBuf::from(
                args.next().ok_or("--capture requires an output path")?,
            ));
        } else if arg == "--simulate-frames" {
            simulate_frames = args
                .next()
                .and_then(|v| v.to_str()?.parse::<u32>().ok())
                .ok_or("--simulate-frames requires a frame count")?;
        } else if arg == "--hold" {
            let keys = args
                .next()
                .ok_or("--hold requires a comma separated key list")?;
            for key in keys.to_string_lossy().split(',') {
                match key {
                    "left" => pad.left = true,
                    "right" => pad.right = true,
                    "down" => pad.down = true,
                    "jump" => pad.jump = true,
                    "kick" => pad.kick = true,
                    other => return Err(format!("--hold unknown key={other}").into()),
                }
            }
        } else if arg == "--release-after" {
            release_after = Some(
                args.next()
                    .and_then(|v| v.to_str()?.parse::<u32>().ok())
                    .ok_or("--release-after requires a frame count")?,
            );
        } else if arg == "--help" {
            println!(
                "Usage: thps2-rs --data-root DIRECTORY [--manifest | --view FILE [--render [--capture PNG]]] [--simulate-frames N] [--hold left,right,down,jump,kick [--release-after N]]\nInspect original resources, step the skater N design frames with an optional held pad released after N frames, or display a static diagnostic with feature viewer."
            );
            return Ok(());
        } else {
            return Err(format!("unknown argument={arg:?}").into());
        }
    }
    if manifest && captured.is_some() {
        return Err("--manifest and --view are mutually exclusive".into());
    }
    if render && captured.is_none() {
        return Err("--render requires --view FILE".into());
    }
    if capture.is_some() && !render {
        return Err("--capture requires --render".into());
    }
    if release_after.is_some() && pad == Pad::default() {
        return Err("--release-after requires --hold".into());
    }
    inspect(
        root.ok_or("--data-root is required (directory containing ALL.PKR)")?,
        manifest,
        captured,
        render,
        capture,
        simulate_frames,
        SessionPad {
            held: pad,
            release_after,
        },
    )
}
