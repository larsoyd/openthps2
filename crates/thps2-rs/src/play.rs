use std::{error::Error, path::Path};

use thps2_data::{
    ground::Pad,
    hangar::{Hangar, Session},
    pkr::Archive,
    raster::Raster,
};

/// The frame as 8-bit RGB, each 16-bit channel widened by repeating its high bits.
pub fn rgb(raster: &Raster) -> Vec<u8> {
    let widen = |v: u16, bits: u16| ((v << (8 - bits)) | (v >> (2 * bits - 8))) as u8;
    raster
        .colour
        .iter()
        .flat_map(|&p| {
            [
                widen(p >> 11, 5),
                widen((p >> 5) & 0x3f, 6),
                widen(p & 0x1f, 5),
            ]
        })
        .collect()
}

/// Runs the session for a number of frames without input and writes the last one as a PPM.
pub fn capture(archive: &Archive, frames: u32, path: &Path) -> Result<(), Box<dyn Error>> {
    let hangar = Hangar::load(archive)?;
    let mut session = Session::start(&hangar)?;
    for _ in 0..frames {
        session.step(&hangar, Pad::default())?;
    }
    let raster = session.compose(&hangar)?;
    let mut ppm = format!("P6\n{} {}\n255\n", raster.width, raster.height).into_bytes();
    ppm.extend(rgb(&raster));
    std::fs::write(path, ppm).map_err(|e| format!("capture={} {e}", path.display()))?;
    let s = &session.run.skater;
    println!(
        "[play] frames={frames} capture={} position={:?} state={:?} score={}",
        path.display(),
        s.position,
        s.state,
        s.panel.score
    );
    Ok(())
}

#[cfg(feature = "viewer")]
pub use window::run;

#[cfg(feature = "viewer")]
mod window {
    use std::error::Error;

    use bevy::{
        asset::RenderAssetUsages,
        prelude::*,
        render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    };
    use thps2_data::{
        ground::Pad,
        hangar::{Hangar, Session},
        pkr::Archive,
    };

    #[derive(Resource)]
    struct Play {
        hangar: Hangar,
        session: Session,
        image: Option<Handle<Image>>,
    }

    /// Keyboard stand ins for the pad until the original bindings are ported.
    fn pad(keys: &ButtonInput<KeyCode>) -> Pad {
        let held = |k: KeyCode| keys.pressed(k);
        Pad {
            left: held(KeyCode::ArrowLeft),
            right: held(KeyCode::ArrowRight),
            up: held(KeyCode::ArrowUp),
            down: held(KeyCode::ArrowDown),
            jump: held(KeyCode::KeyZ) || held(KeyCode::Enter),
            kick: held(KeyCode::KeyX),
            grab: held(KeyCode::KeyC),
            grind: held(KeyCode::KeyV),
            spin_left: held(KeyCode::KeyA),
            spin_right: held(KeyCode::KeyS),
        }
    }

    pub fn run(archive: &Archive) -> Result<(), Box<dyn Error>> {
        let hangar = Hangar::load(archive)?;
        let session = Session::start(&hangar)?;
        println!("[play] keys arrows=move z=ollie x=flip c=grab v=grind a,s=spin enter=menu");
        let mut app = App::new();
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "THPS2 Hangar".into(),
                resolution: (640u32, 480u32).into(),
                resizable: false,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(Time::<Fixed>::from_hz(30.0))
        .insert_resource(Play {
            hangar,
            session,
            image: None,
        })
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, step)
        .add_systems(Update, present);
        if app.run().is_success() {
            Ok(())
        } else {
            Err("play halted".into())
        }
    }

    fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>, mut play: ResMut<Play>) {
        let image = Image::new_fill(
            Extent3d {
                width: 640,
                height: 480,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0, 0, 0, 255],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        let handle = images.add(image);
        commands.spawn(Camera2d);
        commands.spawn(Sprite::from_image(handle.clone()));
        play.image = Some(handle);
    }

    fn step(
        keys: Res<ButtonInput<KeyCode>>,
        mut play: ResMut<Play>,
        mut exit: MessageWriter<AppExit>,
    ) {
        let play = &mut *play;
        if let Err(e) = play.session.step(&play.hangar, pad(&keys)) {
            eprintln!("[play] halted error={e}");
            exit.write(AppExit::error());
        }
    }

    fn present(
        play: Res<Play>,
        mut images: ResMut<Assets<Image>>,
        mut exit: MessageWriter<AppExit>,
    ) {
        let Some(image) = play.image.as_ref().and_then(|h| images.get_mut(h)) else {
            return;
        };
        match play.session.compose(&play.hangar) {
            Ok(raster) => {
                let rgba: Vec<u8> = super::rgb(&raster)
                    .chunks(3)
                    .flat_map(|c| [c[0], c[1], c[2], 255])
                    .collect();
                image.data = Some(rgba);
            }
            Err(e) => {
                eprintln!("[play] halted error={e}");
                exit.write(AppExit::error());
            }
        }
    }
}
