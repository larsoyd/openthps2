use std::{
    error::Error,
    io::Write,
    path::{Path, PathBuf},
};

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

/// One log per run under the user's state directory, keeping the newest twenty.
pub fn log_file() -> Result<(PathBuf, Box<dyn Write + Send + Sync>), String> {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))
        .or_else(|| std::env::var_os("LOCALAPPDATA").map(PathBuf::from))
        .unwrap_or_else(std::env::temp_dir);
    let dir = base.join("openthps2");
    std::fs::create_dir_all(&dir).map_err(|e| format!("log dir={} {e}", dir.display()))?;
    let mut old: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map_err(|e| format!("log dir={} {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("play-"))
        })
        .collect();
    old.sort();
    for path in old.iter().rev().skip(LOGS_KEPT - 1) {
        let _ = std::fs::remove_file(path);
    }
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let path = dir.join(format!("play-{secs}.log"));
    let file = std::fs::File::create(&path).map_err(|e| format!("log={} {e}", path.display()))?;
    Ok((path, Box::new(std::io::BufWriter::new(file))))
}

const LOGS_KEPT: usize = 20;
/// Seconds of window frames each summary line covers.
pub const LOG_PERIOD: f64 = 5.0;

/// Timings of the play window, written as one summary line per period plus event lines.
pub struct RunLog {
    out: Box<dyn Write + Send + Sync>,
    period: f64,
    started: f64,
    now: f64,
    frames: Vec<f64>,
    steps: Vec<f64>,
    composes: Vec<f64>,
}

impl RunLog {
    pub fn new(out: Box<dyn Write + Send + Sync>, period: f64) -> Self {
        Self {
            out,
            period,
            started: 0.0,
            now: 0.0,
            frames: Vec::new(),
            steps: Vec::new(),
            composes: Vec::new(),
        }
    }

    /// A window frame at this many seconds into the run, closing the period once it has passed.
    pub fn frame(&mut self, now: f64) {
        self.frames.push((now - self.now) * 1000.0);
        self.now = now;
        if now - self.started >= self.period {
            self.summary();
        }
    }

    pub fn step(&mut self, ms: f64) {
        self.steps.push(ms);
    }

    pub fn compose(&mut self, ms: f64) {
        self.composes.push(ms);
    }

    pub fn event(&mut self, text: &str) {
        self.line(text);
    }

    fn summary(&mut self) {
        let elapsed = self.now - self.started;
        let (frames, steps, composes) = (
            sorted(std::mem::take(&mut self.frames)),
            sorted(std::mem::take(&mut self.steps)),
            sorted(std::mem::take(&mut self.composes)),
        );
        let text = format!(
            "frames={} fps={:.1} frame_ms_p50={:.1} frame_ms_p99={:.1} frame_ms_max={:.1} \
             steps={} step_ms_max={:.1} composed={} compose_ms_p50={:.1} compose_ms_max={:.1}",
            frames.len(),
            frames.len() as f64 / elapsed,
            rank(&frames, 0.5),
            rank(&frames, 0.99),
            rank(&frames, 1.0),
            steps.len(),
            rank(&steps, 1.0),
            composes.len(),
            rank(&composes, 0.5),
            rank(&composes, 1.0),
        );
        self.line(&text);
        self.started = self.now;
    }

    // a log that cannot be written must not stop the game
    fn line(&mut self, text: &str) {
        let _ =
            writeln!(self.out, "[play] t={:.2} {text}", self.now).and_then(|_| self.out.flush());
    }
}

fn sorted(mut v: Vec<f64>) -> Vec<f64> {
    v.sort_by(f64::total_cmp);
    v
}

/// Nearest rank percentile, zero for an empty list.
fn rank(sorted: &[f64], p: f64) -> f64 {
    let k = (p * sorted.len() as f64).ceil() as usize;
    sorted.get(k.max(1) - 1).copied().unwrap_or(0.0)
}

#[cfg(feature = "viewer")]
pub use window::{StepLimit, add, capture_window, run};

#[cfg(feature = "viewer")]
mod window {
    use std::{error::Error, path::PathBuf};

    use bevy::{
        asset::RenderAssetUsages,
        prelude::*,
        render::{
            render_resource::{Extent3d, TextureDimension, TextureFormat},
            view::screenshot::{Screenshot, ScreenshotCaptured},
        },
        time::TimeUpdateStrategy,
    };
    use std::time::Instant;
    use thps2_data::{
        ground::Pad,
        hangar::{Hangar, Session},
        pkr::Archive,
    };

    use super::{LOG_PERIOD, RunLog};

    /// Fixed steps after which the session stops, so a capture shows a chosen frame.
    #[derive(Resource)]
    pub struct StepLimit(pub u32);

    #[derive(Resource)]
    struct Play {
        hangar: Hangar,
        session: Session,
        image: Option<Handle<Image>>,
        log: RunLog,
        /// A step ran since the last compose, so the frame needs drawing again.
        fresh: bool,
        steps: u32,
        composed: u32,
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
        println!("[play] keys arrows=move z=ollie x=flip c=grab v=grind a,s=spin enter=menu");
        let (path, out) = super::log_file()?;
        println!("[play] log={}", path.display());
        let mut app = App::new();
        app.add_plugins(DefaultPlugins.set(window_plugin()));
        add(&mut app, hangar, RunLog::new(out, LOG_PERIOD))?;
        if app.run().is_success() {
            Ok(())
        } else {
            Err("play halted".into())
        }
    }

    /// Runs one fixed step per window frame up to the limit and saves what the window shows.
    pub fn capture_window(
        archive: &Archive,
        frames: u32,
        path: PathBuf,
    ) -> Result<(), Box<dyn Error>> {
        let hangar = Hangar::load(archive)?;
        let (_, out) = super::log_file()?;
        let mut app = App::new();
        app.add_plugins(DefaultPlugins.set(window_plugin()));
        add(&mut app, hangar, RunLog::new(out, LOG_PERIOD))?;
        app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1))
            .insert_resource(StepLimit(frames))
            .insert_resource(Shot(Some(path)))
            .add_systems(Update, shoot.after(present));
        if app.run().is_success() {
            Ok(())
        } else {
            Err("window capture failed".into())
        }
    }

    fn window_plugin() -> WindowPlugin {
        WindowPlugin {
            primary_window: Some(Window {
                title: "THPS2 Hangar".into(),
                resolution: (640u32, 480u32).into(),
                resizable: false,
                ..default()
            }),
            ..default()
        }
    }

    /// The PNG the window is saved to once the last step is drawn.
    #[derive(Resource)]
    struct Shot(Option<PathBuf>);

    fn shoot(
        mut commands: Commands,
        play: Res<Play>,
        limit: Res<StepLimit>,
        mut shot: ResMut<Shot>,
    ) {
        if play.steps < limit.0 {
            return;
        }
        let Some(path) = shot.0.take() else {
            return;
        };
        let (steps, composed) = (play.steps, play.composed);
        commands.spawn(Screenshot::primary_window()).observe(
            move |event: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
                let saved = event
                    .image
                    .clone()
                    .try_into_dynamic()
                    .map_err(|e| e.to_string())
                    .and_then(|image| image.to_rgb8().save(&path).map_err(|e| e.to_string()));
                match saved {
                    Ok(()) => {
                        println!(
                            "[play] window_steps={steps} composed={composed} capture={}",
                            path.display()
                        );
                        exit.write(AppExit::Success);
                    }
                    Err(e) => {
                        eprintln!("[play] capture={} {e}", path.display());
                        exit.write(AppExit::error());
                    }
                }
            },
        );
    }

    /// The session on a 30 Hz fixed step, drawn into a sprite after each window frame.
    pub fn add(app: &mut App, hangar: Hangar, log: RunLog) -> Result<(), String> {
        let session = Session::start(&hangar)?;
        app.insert_resource(Time::<Fixed>::from_hz(30.0))
            .insert_resource(Play {
                hangar,
                session,
                image: None,
                log,
                fresh: true,
                steps: 0,
                composed: 0,
            })
            .add_systems(Startup, setup)
            .add_systems(FixedUpdate, step)
            .add_systems(Update, present);
        Ok(())
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
        limit: Option<Res<StepLimit>>,
        mut play: ResMut<Play>,
        mut exit: MessageWriter<AppExit>,
    ) {
        let play = &mut *play;
        if limit.as_ref().is_some_and(|l| play.steps >= l.0) {
            return;
        }
        let before = play.session.run.skater.session;
        let frames = play.session.run.frames;
        let started = Instant::now();
        let stepped = play.session.step(&play.hangar, pad(&keys));
        play.log.step(started.elapsed().as_secs_f64() * 1000.0);
        play.fresh = true;
        play.steps += 1;
        if limit.is_some_and(|l| play.steps == l.0) {
            play.log.event(&format!("stopped steps={}", play.steps));
        }
        if let Err(e) = stepped {
            play.log.event(&format!("halted error={e}"));
            eprintln!("[play] halted error={e}");
            exit.write(AppExit::error());
            return;
        }
        let run = &play.session.run;
        let after = run.skater.session;
        if after.ended && !before.ended {
            let row = after.entered.map_or("none".into(), |r| r.to_string());
            let event = format!("run_end score={} table_row={row}", run.skater.panel.score);
            play.log.event(&event);
        }
        if after.menu && !before.menu {
            play.log.event("end_menu");
        }
        if run.frames < frames {
            play.log.event("retry");
        }
    }

    fn present(
        time: Res<Time>,
        mut play: ResMut<Play>,
        mut images: ResMut<Assets<Image>>,
        mut exit: MessageWriter<AppExit>,
    ) {
        let play = &mut *play;
        let image = play.image.as_ref().and_then(|h| images.get_mut(h));
        if let Some(mut image) = image.filter(|_| std::mem::take(&mut play.fresh)) {
            let started = Instant::now();
            let composed = play.session.compose(&play.hangar);
            play.log.compose(started.elapsed().as_secs_f64() * 1000.0);
            play.composed += 1;
            match composed {
                Ok(raster) => {
                    let rgba: Vec<u8> = super::rgb(&raster)
                        .chunks(3)
                        .flat_map(|c| [c[0], c[1], c[2], 255])
                        .collect();
                    image.data = Some(rgba);
                }
                Err(e) => {
                    play.log.event(&format!("halted error={e}"));
                    eprintln!("[play] halted error={e}");
                    exit.write(AppExit::error());
                }
            }
        }
        play.log.frame(time.elapsed_secs_f64());
    }
}
