use bevy::prelude::*;
use thps2_data::{
    collision::{CollisionWorld, Hit},
    ground::{self, DESIGN_FRAME, Outcome, Rules, Skater},
    skater::GROUND_STICK_DISTANCE,
    trig::{Restart, Triggers},
};

use crate::{OriginalScene, SessionAssets};

#[derive(Resource)]
pub(crate) struct SessionCollision(pub(crate) CollisionWorld);

#[derive(Resource)]
pub(crate) struct SessionTriggers(pub(crate) Triggers);

#[derive(Resource, Clone, Copy)]
pub(crate) struct SessionRules(pub(crate) Rules);

#[derive(Component, Debug)]
pub(crate) struct SkaterMotion {
    pub(crate) skater: Skater,
    pub(crate) frames: u32,
    pub(crate) last_ground: Option<(usize, usize, i32)>,
    pub(crate) halted: Option<String>,
}

#[derive(Component, Debug)]
pub(crate) struct SkaterPlacement {
    pub(crate) restart_node: usize,
    pub(crate) restart_position_fixed: [i32; 3],
    pub(crate) position_fixed: [i32; 3],
    pub(crate) source_angles: [i16; 3],
    pub(crate) ground: Hit,
}

/// Native up vector for an unrotated restart, with source Y pointing down.
const RESTART_UP: [i32; 3] = [0, -4096, 0];

fn place(restart: &Restart, collision: &CollisionWorld) -> Result<SkaterPlacement, String> {
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

pub(crate) fn initialize_spawn(
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
        let collision = CollisionWorld::from_scene(&scene.0)?;
        let restart = triggers.restart_named("Start")?;
        let placement = place(restart, &collision)?;
        let motion = SkaterMotion {
            skater: Skater::at_restart(restart.position_fixed, restart.angles),
            frames: 0,
            last_ground: None,
            halted: None,
        };
        Ok::<_, String>((triggers, collision, placement, motion))
    })();
    match result {
        Ok((triggers, collision, placement, motion)) => {
            commands.insert_resource(SessionTriggers(triggers));
            commands.insert_resource(SessionCollision(collision));
            commands.spawn((placement, motion));
        }
        Err(error) => assets.error = Some(format!("Hangar spawn: {error}")),
    }
}

/// One design frame per fixed tick without pad input, stopping at the first unsupported state.
pub(crate) fn advance_ground(
    collision: Option<Res<SessionCollision>>,
    rules: Res<SessionRules>,
    mut motions: Query<&mut SkaterMotion>,
) {
    let Some(collision) = collision else {
        return;
    };
    for mut motion in &mut motions {
        if motion.halted.is_some() {
            continue;
        }
        let frame = motion.frames + 1;
        match ground::ground_step(&collision.0, &mut motion.skater, rules.0, DESIGN_FRAME) {
            Ok(Outcome::Grounded(hit)) => {
                motion.frames = frame;
                motion.last_ground = Some((hit.item_index, hit.face_index, hit.distance_units));
            }
            Ok(Outcome::LeftGround(hit)) => {
                motion.frames = frame;
                motion.halted = Some(format!(
                    "left ground at frame={frame} probe_hit={:?} air_state=unimplemented",
                    hit.map(|h| (h.item_index, h.face_index, h.distance_units))
                ));
            }
            Err(error) => motion.halted = Some(format!("frame={frame} {error}")),
        }
    }
}
