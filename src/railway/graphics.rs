use crate::railway::track::{Track, TrackSegment};
use crate::util::bezier;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

#[derive(Default)]
pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_track_material);
        app.add_systems(Update, spawn_track_meshes);
    }
}

#[derive(Resource)]
pub struct TrackMaterial(pub Handle<ColorMaterial>);

pub fn setup_track_material(mut commands: Commands, mut materials: ResMut<Assets<ColorMaterial>>) {
    commands.insert_resource(TrackMaterial(
        materials.add(ColorMaterial::from(Color::WHITE)),
    ));
}

pub fn spawn_track_meshes(
    mut commands: Commands,
    track: Track,
    q_new_segments: Query<Entity, (With<TrackSegment>, Without<Mesh2d>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    material: Res<TrackMaterial>,
) {
    for seg_ent in q_new_segments.iter() {
        let Some((segment, start_node, end_node)) = track.get_track(seg_ent) else {
            continue;
        };

        let mesh = build_track_mesh(start_node.pos(), segment.p1, segment.p2, end_node.pos());

        commands.entity(seg_ent).insert((
            Mesh2d(meshes.add(mesh)),
            MeshMaterial2d(material.0.clone()),
            Transform::from_xyz(0.0, 0.0, -1.0),
            // bevy::sprite_render::Wireframe2d,
        ));
    }
}

fn build_track_mesh(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut colors: Vec<[f32; 4]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let mut push_quad = |bl: Vec2, br: Vec2, tr: Vec2, tl: Vec2, z: f32, color: [f32; 4]| {
        let start = positions.len() as u32;
        positions.push([bl.x, bl.y, z]);
        positions.push([br.x, br.y, z]);
        positions.push([tr.x, tr.y, z]);
        positions.push([tl.x, tl.y, z]);

        colors.extend_from_slice(&[color, color, color, color]);
        indices.extend_from_slice(&[start, start + 1, start + 2, start + 2, start + 3, start]);
    };

    let segments = 30;
    let ballast_color = [0.15, 0.15, 0.15, 1.0];
    let sleeper_color = [0.35, 0.20, 0.10, 1.0];
    let rail_color = [0.7, 0.7, 0.75, 1.0];

    let ballast_size = 12.0;
    let rail_offset = 7.0;
    let rail_size = 1.0;

    let segment = bezier::build_segment(p0, p1, p2, p3);

    let mut prev_p = segment.position(0.);
    let prev_t = segment.velocity(0.).normalize_or_zero();
    let mut prev_n = Vec2::new(-prev_t.y, prev_t.x);

    for i in 1..=segments {
        let t = i as f32 / segments as f32;
        let curr_p = segment.position(t);
        let curr_t = segment.velocity(t).normalize_or_zero();
        let curr_n = Vec2::new(-curr_t.y, curr_t.x);

        // ballast
        push_quad(
            prev_p + prev_n * ballast_size,
            prev_p - prev_n * ballast_size,
            curr_p - curr_n * ballast_size,
            curr_p + curr_n * ballast_size,
            0.0,
            ballast_color,
        );

        // left rail
        let lp_prev = prev_p + prev_n * rail_offset;
        let lp_curr = curr_p + curr_n * rail_offset;
        push_quad(
            lp_prev + prev_n * rail_size,
            lp_prev - prev_n * rail_size,
            lp_curr - curr_n * rail_size,
            lp_curr + curr_n * rail_size,
            0.2,
            rail_color,
        );

        // right rail
        let rp_prev = prev_p - prev_n * rail_offset;
        let rp_curr = curr_p - curr_n * rail_offset;
        push_quad(
            rp_prev + prev_n * rail_size,
            rp_prev - prev_n * rail_size,
            rp_curr - curr_n * rail_size,
            rp_curr + curr_n * rail_size,
            0.2,
            rail_color,
        );

        prev_p = curr_p;
        prev_n = curr_n;
    }

    // sleepers
    let approx_len = p0.distance(p1) + p1.distance(p2) + p2.distance(p3);
    let sleeper_spacing = 15.0;
    let num_sleepers = (approx_len / sleeper_spacing) as usize;
    let sleeper_width = 10.0;
    let sleeper_length = 1.5;
    for i in 0..=num_sleepers {
        let t = i as f32 / num_sleepers.max(1) as f32;
        let p = segment.position(t);
        let d = segment.velocity(t).normalize_or_zero();
        let n = Vec2::new(-d.y, d.x);

        let s_front = p + d * sleeper_length;
        let s_back = p - d * sleeper_length;

        push_quad(
            s_back + n * sleeper_width,
            s_back - n * sleeper_width,
            s_front - n * sleeper_width,
            s_front + n * sleeper_width,
            0.1,
            sleeper_color,
        );
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; colors.len()]);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}
