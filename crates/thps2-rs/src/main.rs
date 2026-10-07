use bevy::prelude::*;
use sha2::{Digest, Sha256};
use std::{error::Error, path::PathBuf, process::ExitCode};
use thps2_data::pkr::Archive;

#[derive(Resource)]
struct OriginalData(Archive);

#[derive(Resource, Default)]
struct SessionAssets {
    files: Vec<(&'static str, Vec<u8>)>,
    error: Option<String>,
}

const HANGAR_FILES: [&str; 4] = [
    "data/SKHAN.PSX",
    "data/SKHAN_T.TRG",
    "data/HAWK.PSX",
    "data/HAWK.PSH",
];

fn load_hangar(data: Res<OriginalData>, mut assets: ResMut<SessionAssets>) {
    for path in HANGAR_FILES {
        match data.0.read(path) {
            Ok(bytes) => assets.files.push((path, bytes.to_vec())),
            Err(error) => {
                assets.error = Some(error.to_string());
                return;
            }
        }
    }
}

fn inspect(root: PathBuf, manifest: bool) -> Result<(), Box<dyn Error>> {
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
    app.add_plugins(MinimalPlugins)
        .insert_resource(OriginalData(archive))
        .init_resource::<SessionAssets>()
        .add_systems(Startup, load_hangar);
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
    println!(
        "[session] level=Hangar skater=Tony_Hawk resources_loaded={} geometry_decoded=false simulation=false",
        assets.files.len()
    );
    Ok(())
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let mut root = None;
    let mut manifest = false;
    while let Some(arg) = args.next() {
        if arg == "--data-root" {
            root = Some(PathBuf::from(
                args.next().ok_or("--data-root requires a directory")?,
            ));
        } else if arg == "--manifest" {
            manifest = true;
        } else if arg == "--help" {
            println!(
                "Usage: thps2-rs --data-root DIRECTORY [--manifest]\nInspect original resources and load Hangar dependencies through Bevy."
            );
            return Ok(());
        } else {
            return Err(format!("unknown argument={arg:?}").into());
        }
    }
    inspect(
        root.ok_or("--data-root is required (directory containing ALL.PKR)")?,
        manifest,
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
