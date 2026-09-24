use crate::camera::MainCamera;
use crate::controls::Controls;
use crate::railway::track::{Track, TrackNode};
use crate::state_manager::{DespawnWhenMainMenu, GameState};
use bevy::prelude::*;

pub struct DebugRenderPlugin;

#[derive(Resource, Default)]
pub struct TrackDebug {
    pub debug: bool,
}

impl Plugin for DebugRenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_gizmo_group::<TrackGizmosConfig>();
        app.insert_resource(TrackDebug { debug: false });
        app.add_systems(Startup, setup_track_gizmos_config);
        app.add_systems(OnEnter(GameState::Playing), setup_debug_text);
        app.add_systems(
            Update,
            (debug_draw_track, debug_track_text).run_if(|td: Res<TrackDebug>| td.debug),
        );
        app.add_systems(Update, update_debug_setting);
    }
}

#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct TrackGizmosConfig;

pub type TrackGizmos<'w, 's> = Gizmos<'w, 's, TrackGizmosConfig>;

fn setup_track_gizmos_config(mut config_store: ResMut<GizmoConfigStore>, td: Res<TrackDebug>) {
    let (config, _) = config_store.config_mut::<TrackGizmosConfig>();
    config.line = GizmoLineConfig {
        width: 2.0,
        ..default()
    };
    config.depth_bias = -1.0;
    config.enabled = td.debug;
}

fn update_debug_setting(
    controls: Controls,
    mut config_store: ResMut<GizmoConfigStore>,
    mut td: ResMut<TrackDebug>,
    mut s_text: Single<&mut Node, With<DebugTrackText>>,
) {
    if controls.just_pressed(|k| k.debug) {
        td.debug = !td.debug;
        let (config, _) = config_store.config_mut::<TrackGizmosConfig>();
        config.enabled = td.debug;
        if td.debug {
            s_text.display = Display::Flex
        } else {
            s_text.display = Display::None
        }
    }
}

#[derive(Component, Default, Clone)]
struct DebugTrackText;

fn setup_debug_text(mut commands: Commands, td: Res<TrackDebug>) {
    let display = if td.debug {
        Display::Flex
    } else {
        Display::None
    };
    commands.spawn_scene(bsn! {
        Node {
            display,
            position_type: PositionType::Absolute,
            right: px(10),
            top: px(10),
        }
        Text("")
        DebugTrackText
        DespawnWhenMainMenu
    });
}

fn debug_track_text(track: Track, mut s_text: Single<&mut Text, With<DebugTrackText>>) {
    s_text.0 = format!(
        "Segments: {:5}\nNodes:    {:5}",
        track.segments.count(),
        track.nodes.count()
    )
}

fn debug_draw_track(
    track: Track,
    mut gizmos: TrackGizmos,
    q_camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
) {
    let (camera, camera_transform) = *q_camera;
    let Some(viewport_size) = camera.logical_viewport_size() else {
        return;
    };
    let Ok(bottom_left) = camera.viewport_to_world_2d(camera_transform, Vec2::ZERO) else {
        return;
    };
    let Ok(top_right) = camera.viewport_to_world_2d(camera_transform, viewport_size) else {
        return;
    };
    let mut view_min = bottom_left.min(top_right);
    let mut view_max = bottom_left.max(top_right);
    let margin = Vec2::splat(50.0);
    view_min -= margin;
    view_max += margin;
    let intersects = |min1: Vec2, max1: Vec2, min2: Vec2, max2: Vec2| -> bool {
        min1.x <= max2.x && max1.x >= min2.x && min1.y <= max2.y && max1.y >= min2.y
    };
    for (_, segment, start_node, end_node) in track.iter_track() {
        let p0 = start_node.pos();
        let p1 = segment.p1;
        let p2 = segment.p2;
        let p3 = end_node.pos();

        let seg_min = p0.min(p1).min(p2).min(p3);
        let seg_max = p0.max(p1).max(p2).max(p3);

        if intersects(seg_min, seg_max, view_min, view_max) {
            draw_bezier(&mut gizmos, p0, p1, p2, p3, Color::srgb(0.9, 0.9, 0.9));
        }
    }

    for node in track.iter_nodes() {
        if node.pos().x >= view_min.x
            && node.pos().x <= view_max.x
            && node.pos().y >= view_min.y
            && node.pos().y <= view_max.y
        {
            let (dir, color) = match node {
                TrackNode::DeadEnd { tangent, .. } => (tangent, Color::srgb(1.0, 0.0, 0.0)),
                TrackNode::Continuation { normal, .. } => (normal, Color::srgb(0.0, 1.0, 0.0)),
                TrackNode::Junction { normal, .. } => (normal, Color::srgb(0.0, 0.0, 1.0)),
                TrackNode::Crossing { normal, .. } => (normal, Color::srgb(1.0, 1.0, 1.0)),
            };
            gizmos.circle_2d(node.pos(), 5., color);
            gizmos.arrow_2d(
                node.pos(),
                node.pos() + dir * 10.0,
                Color::srgb(1.0, 1.0, 0.),
            );
        }
    }
}

pub fn draw_segments(
    mut gizmos: &mut TrackGizmos,
    segments: &Vec<(Vec2, Vec2, Vec2, Vec2)>,
    color: Color,
) {
    for (sg0, sg1, sg2, sg3) in segments.iter() {
        draw_bezier(&mut gizmos, *sg0, *sg1, *sg2, *sg3, color);
        gizmos.circle_2d(*sg0, 3.0, color);
    }
}

pub fn draw_bezier<T: GizmoConfigGroup>(
    gizmos: &mut Gizmos<T>,
    p0: Vec2,
    p1: Vec2,
    p2: Vec2,
    p3: Vec2,
    color: Color,
) {
    let approx_length = p0.distance(p1) + p1.distance(p2) + p2.distance(p3);
    let calculated_segments = (approx_length / crate::consts::PIXELS_PER_SEGMENT).ceil() as usize;
    let segments = calculated_segments.clamp(10, 256);

    gizmos.curve_2d(
        CubicBezier::new([[p0, p1, p2, p3]]).to_curve().unwrap(),
        (0..=segments).map(|n| n as f32 / segments as f32),
        color,
    );
}
