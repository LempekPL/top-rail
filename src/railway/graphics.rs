use bevy::math::ops::sqrt;
use crate::consts::track::{
    RAIL_OFFSET, RAIL_WIDTH, SLEEPER_HEIGHT, SLEEPER_SPACING, SLEEPER_WIDTH, TRACK_WIDTH,
};
use crate::railway::track::{Track, TrackSegment};
use crate::state_manager::PlayingState;
use crate::util::MeshBuffer;
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::sprite_render::{Material2d, Material2dPlugin};

#[derive(Default)]
pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<BallastMaterial>::default());

        app.add_systems(Startup, (setup_track_material, setup_snap_cursor));
        app.add_systems(
            Update,
            (
                spawn_track_meshes,
                update_snap_cursor_visibility.run_if(state_changed::<PlayingState>),
            ),
        );
    }
}

#[derive(Component)]
pub struct SnapCursor;

fn setup_snap_cursor(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let inner_mesh = meshes.add(Circle::new(sqrt(TRACK_WIDTH)));
    let outer_mesh = meshes.add(Circle::new(TRACK_WIDTH));
    let solid_blue = materials.add(Color::srgb(0.2, 0.6, 1.0));
    let transparent_blue = materials.add(Color::srgba(0.2, 0.6, 1.0, 0.3));
    commands
        .spawn((
            Transform::from_xyz(0.0, 0.0, 10.0),
            Visibility::Hidden,
            SnapCursor,
        ))
        .with_children(|parent| {
            parent.spawn((
                Mesh2d(outer_mesh),
                MeshMaterial2d(transparent_blue),
                Transform::from_xyz(0.0, 0.0, -0.1),
            ));
            parent.spawn((
                Mesh2d(inner_mesh),
                MeshMaterial2d(solid_blue),
                Transform::from_xyz(0.0, 0.0, 0.0),
            ));
        });
}

fn update_snap_cursor_visibility(
    s_railway: Option<Res<State<PlayingState>>>,
    s_cursor: Single<&mut Visibility, With<SnapCursor>>,
) {
    let mut cursor_vis = s_cursor.into_inner();
    if let Some(railway) = s_railway
        && railway.get() == &PlayingState::Build
    {
        *cursor_vis = Visibility::Inherited;
    } else {
        *cursor_vis = Visibility::Hidden;
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct BallastMaterial {}

impl Material2d for BallastMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/ballast.wgsl".into()
    }
}

#[derive(Resource)]
pub struct TrackMaterials {
    pub ballast: Handle<BallastMaterial>,
    pub sleepers: Handle<ColorMaterial>,
    pub rails: Handle<ColorMaterial>,
}

pub fn setup_track_material(
    mut commands: Commands,
    mut ballast_mats: ResMut<Assets<BallastMaterial>>,
    mut color_mats: ResMut<Assets<ColorMaterial>>,
) {
    commands.insert_resource(TrackMaterials {
        ballast: ballast_mats.add(BallastMaterial {}),
        sleepers: color_mats.add(ColorMaterial::from(Color::linear_rgb(0.35, 0.20, 0.10))),
        rails: color_mats.add(ColorMaterial::from(Color::linear_rgb(0.70, 0.70, 0.75))),
    });
}

pub fn spawn_track_meshes(
    mut commands: Commands,
    track: Track,
    q_new_segments: Query<Entity, (With<TrackSegment>, Without<Transform>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    materials: Res<TrackMaterials>,
) {
    for seg_ent in q_new_segments.iter() {
        let Some(curve) = track.get_curve(seg_ent) else {
            continue;
        };

        let (mesh_ballast, mesh_sleepers, mesh_rails) = build_track_mesh_from_curve(curve);

        commands
            .entity(seg_ent)
            .insert((Transform::default(), Visibility::default()))
            .with_children(|parent| {
                parent.spawn((
                    Mesh2d(meshes.add(mesh_ballast)),
                    MeshMaterial2d(materials.ballast.clone()),
                    Transform::from_xyz(0.0, 0.0, -1.0),
                ));
                parent.spawn((
                    Mesh2d(meshes.add(mesh_sleepers)),
                    MeshMaterial2d(materials.sleepers.clone()),
                    Transform::from_xyz(0.0, 0.0, -0.9),
                ));
                parent.spawn((
                    Mesh2d(meshes.add(mesh_rails)),
                    MeshMaterial2d(materials.rails.clone()),
                    Transform::from_xyz(0.0, 0.0, -0.8),
                ));
            });
    }
}

pub fn build_track_mesh_from_curve(cubic_curve: CubicSegment<Vec2>) -> (Mesh, Mesh, Mesh) {
    let p0 = cubic_curve.position(0.0);
    let p1 = cubic_curve.position(1. / 3.);
    let p2 = cubic_curve.position(2. / 3.);
    let p3 = cubic_curve.position(1.0);
    let approx_len = p0.distance(p1) + p1.distance(p2) + p2.distance(p3);
    let segment_count = (approx_len / crate::consts::PIXELS_PER_SEGMENT).ceil() as usize;
    let segment_count = segment_count.clamp(2, 512);

    let mut buf_ballast = MeshBuffer::with_capacity(segment_count);
    let mut buf_rails = MeshBuffer::with_capacity(segment_count * 2);

    let mut prev_p = cubic_curve.position(0.);
    let prev_t = cubic_curve.velocity(0.).normalize_or_zero();
    let mut prev_n = Vec2::new(-prev_t.y, prev_t.x);

    let mut u_accum = 0.0;
    let mut total_length = 0.0;

    for i in 1..=segment_count {
        let t = i as f32 / segment_count as f32;
        let curr_p = cubic_curve.position(t);
        let curr_t = cubic_curve.velocity(t).normalize_or_zero();
        let curr_n = Vec2::new(-curr_t.y, curr_t.x);

        let dist = prev_p.distance(curr_p);
        total_length += dist;

        let u_prev = u_accum;
        u_accum += dist / (TRACK_WIDTH * 2.0);
        let u_curr = u_accum;

        buf_ballast.push_quad_uv(
            prev_p + prev_n * TRACK_WIDTH,
            prev_p - prev_n * TRACK_WIDTH,
            curr_p - curr_n * TRACK_WIDTH,
            curr_p + curr_n * TRACK_WIDTH,
            [u_prev, 0.0],
            [u_prev, 1.0],
            [u_curr, 1.0],
            [u_curr, 0.0],
        );

        let lp_prev = prev_p + prev_n * RAIL_OFFSET;
        let lp_curr = curr_p + curr_n * RAIL_OFFSET;
        buf_rails.push_quad(
            lp_prev + prev_n * RAIL_WIDTH,
            lp_prev - prev_n * RAIL_WIDTH,
            lp_curr - curr_n * RAIL_WIDTH,
            lp_curr + curr_n * RAIL_WIDTH,
        );

        let rp_prev = prev_p - prev_n * RAIL_OFFSET;
        let rp_curr = curr_p - curr_n * RAIL_OFFSET;
        buf_rails.push_quad(
            rp_prev + prev_n * RAIL_WIDTH,
            rp_prev - prev_n * RAIL_WIDTH,
            rp_curr - curr_n * RAIL_WIDTH,
            rp_curr + curr_n * RAIL_WIDTH,
        );

        prev_p = curr_p;
        prev_n = curr_n;
    }

    let num_sleepers = (total_length / SLEEPER_SPACING) as usize;
    let mut buf_sleepers = MeshBuffer::with_capacity(num_sleepers + 1);

    for i in 0..=num_sleepers {
        let t = i as f32 / num_sleepers.max(1) as f32;
        let p = cubic_curve.position(t);
        let d = cubic_curve.velocity(t).normalize_or_zero();
        let n = Vec2::new(-d.y, d.x);

        let s_front = p + d * SLEEPER_HEIGHT;
        let s_back = p - d * SLEEPER_HEIGHT;

        buf_sleepers.push_quad(
            s_back + n * SLEEPER_WIDTH,
            s_back - n * SLEEPER_WIDTH,
            s_front - n * SLEEPER_WIDTH,
            s_front + n * SLEEPER_WIDTH,
        );
    }

    (
        buf_ballast.to_mesh(),
        buf_sleepers.to_mesh(),
        buf_rails.to_mesh(),
    )
}

pub fn tint(mesh: &mut Mesh, color_fn: impl Fn([f32; 4]) -> [f32; 4]) {
    if let Some(bevy::mesh::VertexAttributeValues::Float32x4(colors)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_COLOR)
    {
        for color in colors.iter_mut() {
            *color = color_fn(*color);
        }
    }
}
