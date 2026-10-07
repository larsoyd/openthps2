use bevy::prelude::*;
use thps2_data::projection::{ClippedVertex, Projection, StaticView};

use crate::{HangarVertices, OriginalScene, SceneInstance, SessionAssets};

#[derive(Resource)]
pub(crate) struct CapturedView {
    pub(crate) view: StaticView,
    pub(crate) viewport: [u16; 10],
}

impl CapturedView {
    pub(crate) fn parse(text: &str) -> Result<Self, String> {
        let mut tokens = text.split_whitespace();
        fn record<T: std::str::FromStr, const N: usize>(
            tokens: &mut std::str::SplitWhitespace<'_>,
            label: &str,
        ) -> Result<[T; N], String> {
            if tokens.next() != Some(label) {
                return Err(format!("view expected {label} record"));
            }
            let values = (0..N)
                .map(|_| {
                    tokens
                        .next()
                        .ok_or_else(|| format!("view truncated {label}"))?
                        .parse()
                        .map_err(|_| format!("view invalid {label} value"))
                })
                .collect::<Result<Vec<T>, _>>()?;
            values
                .try_into()
                .map_err(|_| format!("view invalid {label} length"))
        }
        let position = record::<i32, 3>(&mut tokens, "camera")?;
        let matrix = record::<i16, 9>(&mut tokens, "matrix")?;
        let viewport = record::<u16, 10>(&mut tokens, "viewport")?;
        if tokens.next().is_some() {
            return Err("view unexpected trailing data".into());
        }
        let captured = Self {
            view: StaticView::new(
                position,
                std::array::from_fn(|row| std::array::from_fn(|col| matrix[row * 3 + col])),
            ),
            viewport,
        };
        captured.projection(false)?;
        Ok(captured)
    }

    pub(crate) fn projection(&self, extended_far: bool) -> Result<Projection, String> {
        let v = self.viewport.map(f32::from);
        Projection::new(
            [v[2], v[3], v[0], v[1]],
            [v[8], v[9]],
            v[7],
            [v[4], if extended_far { 32767.0 } else { v[5] } * 1.5],
        )
    }
}

#[derive(Component)]
pub(crate) struct ProjectedInstance(pub(crate) Vec<ClippedVertex>);

pub(crate) fn project_hangar(
    mut commands: Commands,
    captured: Option<Res<CapturedView>>,
    scene: Option<Res<OriginalScene>>,
    vertices: Option<Res<HangarVertices>>,
    instances: Query<(Entity, &SceneInstance)>,
    mut assets: ResMut<SessionAssets>,
) {
    let (Some(captured), Some(scene), Some(vertices)) = (captured, scene, vertices) else {
        return;
    };
    for (entity, instance) in &instances {
        let item = &scene.0.items[instance.item_index];
        let result = (|| {
            if item.flags[0] & !0x2000 != 0 || item.flags[1] != 0 {
                return Err(format!("unsupported static item flags={:?}", item.flags));
            }
            let transform = captured
                .view
                .item_transform(item.position_fixed, item.rotation)?;
            captured
                .projection(item.flags[0] & 0x2000 != 0)?
                .project_model(&vertices.0[item.model_index as usize], &transform)
        })();
        match result {
            Ok(projected) => {
                commands.entity(entity).insert(ProjectedInstance(projected));
            }
            Err(error) => {
                assets.error = Some(format!("item={} {error}", instance.item_index));
                return;
            }
        }
    }
}
