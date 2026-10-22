use std::{error::Error, path::PathBuf};

use bevy::{
    asset::RenderAssetUsages,
    core_pipeline::tonemapping::Tonemapping,
    image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor},
    mesh::PrimitiveTopology,
    prelude::*,
    render::{
        render_resource::{Extent3d, Face, TextureDimension, TextureFormat},
        view::screenshot::{Screenshot, ScreenshotCaptured},
    },
};
use thps2_data::projection::face_outside_clip_volume;

use crate::{
    HangarDraws, HangarSurfaces, OriginalScene, SceneInstance,
    view::{CapturedView, ProjectedInstance},
};

#[derive(Resource)]
struct RenderInput {
    meshes: Vec<Mesh>,
    images: Vec<Image>,
    viewport: [u16; 10],
}

#[derive(Resource)]
struct Capture {
    path: Option<PathBuf>,
    frames: u32,
}

pub fn run(world: &mut World, capture: Option<PathBuf>) -> Result<(), Box<dyn Error>> {
    let viewport = world.resource::<CapturedView>().viewport;
    if viewport[2] != 0
        || viewport[3] != 0
        || u32::from(viewport[8]) * 2 != u32::from(viewport[0])
        || u32::from(viewport[9]) * 2 != u32::from(viewport[1])
    {
        return Err("diagnostic renderer requires a centered, zero-origin viewport".into());
    }
    let images: Vec<_> = world
        .resource::<HangarSurfaces>()
        .0
        .iter()
        .map(|surface| {
            let mut rgba = Vec::with_capacity(surface.pixels.len() * 4);
            for &word in &surface.pixels {
                let expand = |channel: u16| ((channel << 3) | (channel >> 2)) as u8;
                rgba.extend_from_slice(&[
                    expand((word >> 10) & 31),
                    expand((word >> 5) & 31),
                    expand(word & 31),
                    if word & 0x8000 != 0 { 255 } else { 0 },
                ]);
            }
            let mut image = Image::new(
                Extent3d {
                    width: surface.width as u32,
                    height: surface.height as u32,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                rgba,
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::default(),
            );
            image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: ImageAddressMode::Repeat,
                address_mode_v: ImageAddressMode::Repeat,
                ..ImageSamplerDescriptor::nearest()
            });
            image
        })
        .collect();
    let mut positions: Vec<Vec<[f32; 3]>> = vec![Vec::new(); images.len()];
    let mut uvs: Vec<Vec<[f32; 2]>> = vec![Vec::new(); images.len()];
    let mut query = world.query::<(&SceneInstance, &ProjectedInstance)>();
    let scene = &world.resource::<OriginalScene>().0;
    let draws = &world.resource::<HangarDraws>().0;
    let mut triangles = 0;
    let mut rejected = 0;
    for (instance, projected) in query.iter(world) {
        for draw in &draws[scene.items[instance.item_index].model_index as usize] {
            if draw.corners.len() != 3 {
                return Err("diagnostic renderer requires split triangles".into());
            }
            let corners = draw
                .corners
                .iter()
                .map(|c| projected.0[c.vertex as usize])
                .collect::<Vec<_>>();
            if face_outside_clip_volume(&corners)? {
                rejected += 1;
                continue;
            }
            let texture = draw
                .texture_index
                .ok_or("diagnostic renderer does not yet support untextured faces")?
                as usize;
            if texture >= images.len() {
                return Err("diagnostic renderer texture index out of range".into());
            }
            for corner in &draw.corners {
                let v = projected.0[corner.vertex as usize].projected;
                positions[texture].push([
                    (v.pixel[0] - viewport[8] as f32) / v.scale,
                    -(v.pixel[1] - viewport[9] as f32) / v.scale,
                    -v.depth,
                ]);
                uvs[texture].push(corner.uv.ok_or("diagnostic renderer missing corner UV")?);
            }
            triangles += 1;
        }
    }
    let used_textures = positions.iter().filter(|p| !p.is_empty()).count();
    let meshes = positions
        .into_iter()
        .zip(uvs)
        .map(|(positions, uvs)| {
            let normals = vec![[0.0, 0.0, 1.0]; positions.len()];
            Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::default(),
            )
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        })
        .collect();
    println!(
        "[viewer] submitted_triangles={triangles} clip_rejected={rejected} available_textures={} used_textures={used_textures} lighting=unlit culling=native_clockwise blending=alpha_mask sampler=nearest_repeat ordering=depth_buffer animation=false simulation=false",
        images.len()
    );
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "THPS2 Hangar static diagnostic - not playable".into(),
            resolution: (viewport[0] as u32, viewport[1] as u32).into(),
            resizable: false,
            ..default()
        }),
        ..default()
    }))
    .insert_resource(ClearColor(Color::srgb(0.1, 0.1, 0.1)))
    .insert_resource(RenderInput {
        meshes,
        images,
        viewport,
    })
    .insert_resource(Capture {
        path: capture,
        frames: 0,
    })
    .add_systems(Startup, setup)
    .add_systems(Update, capture_or_exit);
    if app.run().is_success() {
        Ok(())
    } else {
        Err("diagnostic renderer failed".into())
    }
}

fn setup(
    mut commands: Commands,
    mut input: ResMut<RenderInput>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let v = input.viewport;
    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: 2.0 * (v[1] as f32 / (2.0 * v[7] as f32)).atan(),
            aspect_ratio: v[0] as f32 / v[1] as f32,
            near: v[4] as f32,
            far: 32767.0 * 1.5,
            near_clip_plane: Vec4::new(0.0, 0.0, -1.0, -(v[4] as f32)),
        }),
        Tonemapping::Linear,
        Msaa::Off,
    ));
    let mesh_data = std::mem::take(&mut input.meshes);
    let image_data = std::mem::take(&mut input.images);
    for (mesh, image) in mesh_data.into_iter().zip(image_data) {
        if mesh.count_vertices() == 0 {
            continue;
        }
        let material = materials.add(StandardMaterial {
            base_color_texture: Some(images.add(image)),
            unlit: true,
            // Native positive screen area becomes clockwise after the Y-axis conversion.
            cull_mode: Some(Face::Front),
            alpha_mode: AlphaMode::Mask(0.5),
            ..default()
        });
        commands.spawn((Mesh3d(meshes.add(mesh)), MeshMaterial3d(material)));
    }
}

fn capture_or_exit(
    mut commands: Commands,
    mut capture: ResMut<Capture>,
    keys: Res<ButtonInput<KeyCode>>,
    mut exit: MessageWriter<AppExit>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
    capture.frames += 1;
    if capture.frames != 120 {
        return;
    }
    if let Some(path) = capture.path.take() {
        commands.spawn(Screenshot::primary_window()).observe(
            move |event: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
                let result = event
                    .image
                    .clone()
                    .try_into_dynamic()
                    .map_err(|error| error.to_string())
                    .and_then(|image| {
                        image
                            .to_rgb8()
                            .save(&path)
                            .map_err(|error| error.to_string())
                    });
                match result {
                    Ok(()) => {
                        println!("[capture] path={}", path.display());
                        exit.write(AppExit::Success);
                    }
                    Err(error) => {
                        eprintln!("[capture] {error}");
                        exit.write(AppExit::error());
                    }
                }
            },
        );
    }
}
