use bevy::prelude::*;
use sha2::{Digest, Sha256};
use std::{error::Error, path::PathBuf, process::ExitCode};
use thps2_data::draw::{DrawFace, FaceOverride, PreparedVertex, prepare_face, prepare_vertices};
use thps2_data::materials::{Surface1555, Texture, TextureSource, resolve_textures};
use thps2_data::pkr::Archive;
use thps2_data::psx::Scene;
mod view;
use view::{CapturedView, ProjectedInstance, project_hangar};

#[derive(Resource)]
struct OriginalData(Archive);

#[derive(Resource)]
struct OriginalScene(Scene);

#[derive(Resource)]
struct SkaterScene(Scene);

#[derive(Resource)]
struct HangarTextures(Vec<Texture>);

#[derive(Resource)]
struct HangarSurfaces(Vec<Surface1555>);

#[derive(Resource)]
struct HangarDraws(Vec<Vec<DrawFace>>);

#[derive(Resource)]
struct HangarVertices(Vec<Vec<PreparedVertex>>);

#[derive(Component)]
struct SceneInstance {
    item_index: usize,
}

#[derive(Resource, Default)]
struct SessionAssets {
    files: Vec<(&'static str, Vec<u8>)>,
    error: Option<String>,
}

const HANGAR_FILES: [&str; 5] = [
    "data/SKHAN.PSX",
    "data/SKHAN_T.TRG",
    "data/HAWK.PSX",
    "data/HAWK.PSH",
    "data/SKHAN_L.PSX",
];

fn load_hangar(mut commands: Commands, data: Res<OriginalData>, mut assets: ResMut<SessionAssets>) {
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

fn inspect(
    root: PathBuf,
    manifest: bool,
    captured: Option<CapturedView>,
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
        .init_resource::<SessionAssets>()
        .add_systems(Startup, (load_hangar, project_hangar).chain());
    app.update();
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
        "[scene] items={} models={} placed_faces={} textures={} raw_chunks={} rendering=false simulation=false",
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
        "[materials] resolved={} bitmap={} indexed={} gpu_upload=false",
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
        "[surfaces] format=ARGB1555 count={} transparent_pixels={transparent} gpu_upload=false",
        surfaces.len()
    );
    let vertices = &app.world().resource::<HangarVertices>().0;
    println!(
        "[vertices] records={} screen_offsets={} camera_projection=false",
        vertices.iter().map(Vec::len).sum::<usize>(),
        vertices
            .iter()
            .flatten()
            .filter(|v| matches!(v, PreparedVertex::ScreenOffset { .. }))
            .count()
    );
    let draws = &app.world().resource::<HangarDraws>().0;
    println!(
        "[draws] models={} records={} overrides=identity textured_quad_split=true gpu_submission=false",
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
    Ok(())
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let mut root = None;
    let mut manifest = false;
    let mut captured = None;
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
        } else if arg == "--help" {
            println!(
                "Usage: thps2-rs --data-root DIRECTORY [--manifest | --view FILE]\nInspect original resources and optionally project Hangar from a captured view."
            );
            return Ok(());
        } else {
            return Err(format!("unknown argument={arg:?}").into());
        }
    }
    if manifest && captured.is_some() {
        return Err("--manifest and --view are mutually exclusive".into());
    }
    inspect(
        root.ok_or("--data-root is required (directory containing ALL.PKR)")?,
        manifest,
        captured,
    )
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("[error] {error}");
            ExitCode::FAILURE
        }
    }
}
