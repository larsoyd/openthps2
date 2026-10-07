use bevy::prelude::*;
use thps2_data::{
    collision::{CollisionWorld, Hit},
    skater::GROUND_STICK_DISTANCE,
    trig::{Restart, Triggers},
};

use crate::{OriginalScene, SessionAssets};

#[derive(Resource)]
pub(crate) struct SessionCollision(pub(crate) CollisionWorld);

#[derive(Resource)]
pub(crate) struct SessionTriggers(pub(crate) Triggers);

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
        let placement = place(triggers.restart_named("Start")?, &collision)?;
        Ok::<_, String>((triggers, collision, placement))
    })();
    match result {
        Ok((triggers, collision, placement)) => {
            commands.insert_resource(SessionTriggers(triggers));
            commands.insert_resource(SessionCollision(collision));
            commands.spawn(placement);
        }
        Err(error) => assets.error = Some(format!("Hangar spawn: {error}")),
    }
}
