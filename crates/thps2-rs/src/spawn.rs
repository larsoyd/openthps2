use bevy::prelude::*;
use thps2_data::{
    anim::AnimationTable,
    collision::{CollisionWorld, Hit},
    ground::{Level, Outcome, Pad, Rules, Run},
    psx::Scene,
    script::TrickScripts,
    skater::GROUND_STICK_DISTANCE,
    trig::{Restart, Triggers},
};

use crate::{OriginalScene, SessionAssets};

#[derive(Resource)]
pub struct SessionCollision(pub CollisionWorld);

#[derive(Resource)]
pub struct SessionTriggers(pub Triggers);

/// Skater bank animation table from SK2ANIM.PSX.
#[derive(Resource)]
pub struct SessionAnimations(pub AnimationTable);

#[derive(Resource)]
pub struct SessionScripts(pub TrickScripts);

#[derive(Resource, Clone, Copy)]
pub struct SessionRules(pub Rules);

/// Pad held for every stepped frame, released after an optional frame count.
#[derive(Resource, Clone, Copy, Default)]
pub struct SessionPad {
    pub held: Pad,
    pub release_after: Option<u32>,
}

impl SessionPad {
    fn at(&self, frame: u32) -> Pad {
        match self.release_after {
            Some(limit) if frame > limit => Pad::default(),
            _ => self.held,
        }
    }
}

#[derive(Component, Debug)]
pub struct SkaterMotion {
    pub run: Run,
    pub frames: u32,
    pub last_ground: Option<(usize, usize, i32)>,
    pub halted: Option<String>,
}

#[derive(Component, Debug)]
pub struct SkaterPlacement {
    pub restart_node: usize,
    pub restart_position_fixed: [i32; 3],
    pub position_fixed: [i32; 3],
    pub source_angles: [i16; 3],
    pub ground: Hit,
}

/// Native up vector for an unrotated restart, with source Y pointing down.
const RESTART_UP: [i32; 3] = [0, -4096, 0];

pub fn place(restart: &Restart, collision: &CollisionWorld) -> Result<SkaterPlacement, String> {
    let ground = collision
        .ground_probe(restart.position_fixed, RESTART_UP)?
        .ok_or("spawn has no supporting ground")?;
    if ground.distance_units > GROUND_STICK_DISTANCE {
        return Err(format!(
            "spawn ground distance={} outside initial standing range",
            ground.distance_units
        ));
    }
    let position_fixed = collision.stand(restart.position_fixed, RESTART_UP, &ground)?;
    Ok(SkaterPlacement {
        restart_node: restart.node_index,
        restart_position_fixed: restart.position_fixed,
        position_fixed,
        source_angles: restart.angles,
        ground,
    })
}

pub fn initialize_spawn(
    mut commands: Commands,
    scene: Option<Res<OriginalScene>>,
    mut assets: ResMut<SessionAssets>,
) {
    if assets.error.is_some() {
        return;
    }
    let Some(scene) = scene else {
        return;
    };
    let result = (|| {
        let bytes = &assets
            .files
            .iter()
            .find(|(name, _)| *name == "data/SKHAN_T.TRG")
            .ok_or("spawn missing Hangar triggers")?
            .1;
        let triggers = Triggers::parse(bytes)?;
        let bank = &assets
            .files
            .iter()
            .find(|(name, _)| *name == "data/SK2ANIM.PSX")
            .ok_or("spawn missing skater animation bank")?
            .1;
        let table = AnimationTable::from_scene(&Scene::parse(bank)?)?;
        let tricks = &assets
            .files
            .iter()
            .find(|(name, _)| *name == "data/TRICKS.BIN")
            .ok_or("spawn missing trick scripts")?
            .1;
        let scripts = TrickScripts::parse(tricks.clone())?;
        let collision = CollisionWorld::from_scene(&scene.0)?;
        let restart = triggers.restart_named("Start")?;
        let placement = place(restart, &collision)?;
        let motion = SkaterMotion {
            run: Run::start(&table, restart.position_fixed, restart.angles)?,
            frames: 0,
            last_ground: None,
            halted: None,
        };
        Ok::<_, String>((triggers, table, scripts, collision, placement, motion))
    })();
    match result {
        Ok((triggers, table, scripts, collision, placement, motion)) => {
            commands.insert_resource(SessionTriggers(triggers));
            commands.insert_resource(SessionAnimations(table));
            commands.insert_resource(SessionScripts(scripts));
            commands.insert_resource(SessionCollision(collision));
            commands.spawn((placement, motion));
        }
        Err(error) => assets.error = Some(format!("Hangar spawn: {error}")),
    }
}

/// One run frame per fixed tick with the held pad, stopping at the first unsupported contact.
pub fn advance_ground(
    collision: Option<Res<SessionCollision>>,
    animations: Option<Res<SessionAnimations>>,
    scripts: Option<Res<SessionScripts>>,
    triggers: Option<Res<SessionTriggers>>,
    rules: Res<SessionRules>,
    pad: Res<SessionPad>,
    mut motions: Query<&mut SkaterMotion>,
) {
    let (Some(collision), Some(animations), Some(scripts), Some(triggers)) =
        (collision, animations, scripts, triggers)
    else {
        return;
    };
    let level = Level {
        world: &collision.0,
        table: &animations.0,
        tricks: &scripts.0,
        triggers: &triggers.0,
    };
    for mut motion in &mut motions {
        if motion.halted.is_some() {
            continue;
        }
        let frame = motion.frames + 1;
        match motion.run.frame(&level, rules.0, pad.at(frame)) {
            Ok(Outcome::Grounded(hit) | Outcome::Landed(hit)) => {
                motion.frames = frame;
                motion.last_ground = Some((hit.item_index, hit.face_index, hit.distance_units));
            }
            Ok(Outcome::LeftGround(_) | Outcome::Airborne) => motion.frames = frame,
            Err(error) => motion.halted = Some(format!("frame={frame} {error}")),
        }
    }
}
