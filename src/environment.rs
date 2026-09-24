use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::sprite_render::{Material2d, Material2dPlugin};

pub struct EnvironmentPlugin;

impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<TerrainMaterial>::default());

        app.add_systems(Startup, setup_environment);
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct TerrainMaterial {}

impl Material2d for TerrainMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/terrain.wgsl".into()
    }
}


fn setup_environment(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut terrain_mats: ResMut<Assets<TerrainMaterial>>,
) {
    let map_size = 100_000.0;
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(map_size, map_size))),
        MeshMaterial2d(terrain_mats.add(TerrainMaterial {})),
        Transform::from_xyz(0.0, 0.0, -100.0),
    ));
}