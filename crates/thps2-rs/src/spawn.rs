use bevy::prelude::*;
use thps2_data::{
    collision::{CollisionWorld, Hit},
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

fn place(restart: &Restart, collision: &CollisionWorld) -> Result<SkaterPlacement, String> {
    let mut above = restart.position_fixed;
    let mut below = restart.position_fixed;
    above[1] = above[1]
        .checked_sub(70 * 4096)
        .ok_or("spawn ground probe overflow")?;
    below[1] = below[1]
        .checked_add(186 * 4096)
        .ok_or("spawn ground probe overflow")?;
    let ground = collision
        .query(above, below)?
        .ok_or("spawn has no supporting ground")?;
    if ground.distance_units >= 116 {
        return Err(format!(
            "spawn ground distance={} outside initial standing range",
            ground.distance_units
        ));
    }
    let mut position_fixed = ground.position_fixed;
    position_fixed[1] = position_fixed[1]
        .checked_sub(30 * 4096)
        .ok_or("spawn standing position overflow")?;
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
