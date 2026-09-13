use crate::camera;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

#[derive(Default)]
pub struct TrackPlugin;

const SNAP_RADIUS: f32 = 10.0;
const LINE_SEGMENT_LENGTH: f32 = 100.0;
const MIN_RADIUS: f32 = 50.0;

impl Plugin for TrackPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TrackBuilder>();
        app.add_systems(Update, (build_track, debug_draw_track));
    }
}

#[derive(Resource, Default)]
pub struct TrackBuilder {
    pub is_dragging: bool,
    pub start_pos: Vec2,
    pub start_tangent: Option<Vec2>,
    pub start_node: Option<Entity>,
}

#[derive(Component, Debug, Clone)]
pub struct TrackSegment {
    pub p0: Vec2,
    pub p1: Vec2,
    pub p2: Vec2,
    pub p3: Vec2,
    pub start_node: TrackConnection,
    pub end_node: TrackConnection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackConnection {
    Node(Entity),
    Segment(Entity),
    None,
}

#[derive(Component, Debug, Clone)]
pub struct TrackNode {
    pub outward_tangent: Vec2,
    pub outgoing_tracks: Vec<Entity>,
    pub active_track_index: usize,
}

impl TrackNode {
    pub fn new(outward_tangent: Vec2) -> Self {
        Self {
            outward_tangent,
            outgoing_tracks: Vec::new(),
            active_track_index: 0,
        }
    }
}

fn build_track(
    mut commands: Commands,
    mut gizmos: Gizmos,
    mut builder: ResMut<TrackBuilder>,
    s_window: Single<&Window, With<PrimaryWindow>>,
    s_camera: Single<(&Camera, &GlobalTransform), With<camera::MainCamera>>,
    r_mouse: Res<ButtonInput<MouseButton>>,
    q_nodes: Query<(Entity, &GlobalTransform, &TrackNode)>,
) {
    let (camera, camera_transform) = *s_camera;
    let Some(cursor_world_pos) = s_window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world_2d(camera_transform, cursor).ok())
    else {
        return;
    };

    if r_mouse.just_pressed(MouseButton::Left) {
        let mut snapped = false;
        for (entity, transform, node) in q_nodes.iter() {
            let node_pos = transform.translation().truncate();
            if node_pos.distance(cursor_world_pos) < SNAP_RADIUS {
                builder.start_pos = node_pos;
                builder.start_tangent = Some(node.outward_tangent);
                builder.start_node = Some(entity);
                snapped = true;
                break;
            }
        }

        if !snapped {
            builder.start_pos = cursor_world_pos;
            builder.start_tangent = None;
            builder.start_node = None;
        }
        builder.is_dragging = true;
    }

    if builder.is_dragging {
        let p0 = builder.start_pos;
        let p3 = cursor_world_pos;
        let chord = p3 - p0;
        let dist = chord.length();
        let chord_dir = chord.normalize_or_zero();

        let mut split_segments = Vec::new();
        let mut out_tangent_p3 = chord_dir;

        let mut is_curve = false;
        let mut straight_line_dir = chord_dir;

        if let Some(mut t0) = builder.start_tangent {
            // if t0.dot(chord_dir) < 0.0 {
            //     t0 = -t0;
            // }
            let n0 = Vec2::new(-t0.y, t0.x);
            let d = chord.dot(n0);
            if d.abs() > 0.1 {
                is_curve = true;

                let turn_dir = d.signum();
                let mut radius = (chord.length_squared() / (2.0 * d.abs())).abs();
                radius = radius.max(MIN_RADIUS);
                let center = p0 + n0 * radius * turn_dir;
                let start_angle = (p0 - center).to_angle();
                let end_angle = (p3 - center).to_angle();

                let mut sweep = end_angle - start_angle;
                if d > 0.0 && sweep < 0.0 {
                    sweep += std::f32::consts::TAU;
                }
                if d < 0.0 && sweep > 0.0 {
                    sweep -= std::f32::consts::TAU;
                }
                sweep = sweep.clamp(-std::f32::consts::TAU * 0.95, std::f32::consts::TAU * 0.95);

                let arc_length = radius * sweep.abs();
                let splits_by_length = (arc_length / LINE_SEGMENT_LENGTH).ceil() as usize;
                let max_sweep = std::f32::consts::FRAC_PI_2;
                let splits_by_angle = (sweep.abs() / max_sweep).ceil() as usize;
                let num_splits = splits_by_length.max(splits_by_angle).max(1);
                let step = sweep / num_splits as f32;
                let k = (4.0 / 3.0) * (step.abs() / 4.0).tan();

                let mut current_angle = start_angle;
                for _ in 0..num_splits {
                    let next_angle = current_angle + step;

                    let tan0 = Vec2::new(-current_angle.sin(), current_angle.cos()) * turn_dir;
                    let tan1 = Vec2::new(-next_angle.sin(), next_angle.cos()) * turn_dir;

                    let q0 = center + Vec2::new(current_angle.cos(), current_angle.sin()) * radius;
                    let q3 = center + Vec2::new(next_angle.cos(), next_angle.sin()) * radius;

                    let q1 = q0 + tan0 * (k * radius);
                    let q2 = q3 - tan1 * (k * radius);

                    split_segments.push((q0, q1, q2, q3));
                    current_angle = next_angle;
                    out_tangent_p3 = tan1;
                }
            } else {
                straight_line_dir = t0;
            }
        }

        if !is_curve {
            let num_splits = (dist / LINE_SEGMENT_LENGTH).ceil().max(1.0) as usize;
            let step_len = dist / num_splits as f32;
            for i in 0..num_splits {
                let q0 = p0 + straight_line_dir * (i as f32 * step_len);
                let q3 = p0 + straight_line_dir * ((i + 1) as f32 * step_len);
                split_segments.push((q0, q0.lerp(q3, 0.33), q0.lerp(q3, 0.66), q3));
            }
            out_tangent_p3 = straight_line_dir;
        }

        if p0.distance(p3) > SNAP_RADIUS {
            for (sq0, sq1, sq2, sq3) in &split_segments {
                draw_bezier(
                    &mut gizmos,
                    *sq0,
                    *sq1,
                    *sq2,
                    *sq3,
                    Color::srgb(0.2, 1.0, 0.1),
                );
                gizmos.circle_2d(*sq0, 3.0, Color::srgb(1.0, 1.0, 1.0));
            }
        }

        if r_mouse.just_released(MouseButton::Left) {
            builder.is_dragging = false;
            if p0.distance(p3) < SNAP_RADIUS { return; }

            let first = split_segments.first().unwrap();
            let last = split_segments.last().unwrap();
            if let Some(entity) = builder.start_node {
                commands.entity(entity).despawn();
            } else {
                commands.spawn((
                    TrackNode::new((p0 - first.1).normalize_or_zero()),
                    Transform::from_translation(p0.extend(0.)),
                ));
            }
            commands.spawn((
                TrackNode::new((last.3 - last.2).normalize_or_zero()),
                Transform::from_translation(last.3.extend(0.)),
            ));

            let mut spawned_segments = Vec::new();
            for (q0, q1, q2, q3) in split_segments.iter() {
                let id = commands.spawn(TrackSegment {
                    p0: *q0, p1: *q1, p2: *q2, p3: *q3,
                    start_node: TrackConnection::None,
                    end_node: TrackConnection::None,
                }).id();
                spawned_segments.push(id);
            }

            for i in 0..spawned_segments.len() {
                let current_ent = spawned_segments[i];

                let prev_conn = if i > 0 {
                    TrackConnection::Segment(spawned_segments[i - 1])
                } else {
                    TrackConnection::None
                };

                let next_conn = if i < spawned_segments.len() - 1 {
                    TrackConnection::Segment(spawned_segments[i + 1])
                } else {
                    TrackConnection::None
                };

                commands.entity(current_ent).insert(TrackSegment {
                    p0: split_segments[i].0,
                    p1: split_segments[i].1,
                    p2: split_segments[i].2,
                    p3: split_segments[i].3,
                    start_node: prev_conn,
                    end_node: next_conn,
                });
            }

            // if let Some(node_entity) = builder.start_node {
            //     if let Ok((_, _, mut node)) = q_nodes.get_mut(node_entity) {
            //         node.outgoing_tracks.push(spawned_segments[0]);
            //     }
            // }
        }
    }
}

fn debug_draw_track(
    q_segments: Query<&TrackSegment>,
    q_nodes: Query<(&GlobalTransform, &TrackNode)>,
    mut gizmos: Gizmos,
) {
    for segment in q_segments.iter() {
        draw_bezier(
            &mut gizmos,
            segment.p0,
            segment.p1,
            segment.p2,
            segment.p3,
            Color::srgb(0.9, 0.9, 0.9),
        );
        gizmos.circle_2d(
            segment.p3,
            3.0,
            Color::srgb(1.0, 1.0, 1.0),
        );
    }

    for (transform, node) in q_nodes.iter() {
        gizmos.circle_2d(
            transform.translation().truncate(),
            SNAP_RADIUS,
            Color::srgb(0.2, 0.5, 1.0),
        );

        gizmos.arrow_2d(
            transform.translation().xy(),
            transform.translation().xy() + node.outward_tangent * 10.0,
            Color::srgb(1.0, 1.0, 0.2),
        );
    }
}

pub fn draw_bezier(gizmos: &mut Gizmos, p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, color: Color) {
    let pixels_per_segment = 15.0;

    let approx_length = p0.distance(p1) + p1.distance(p2) + p2.distance(p3);
    let calculated_segments = (approx_length / pixels_per_segment).ceil() as usize;
    let segments = calculated_segments.clamp(10, 256);
    let mut prev_point = p0;
    for i in 1..=segments {
        let t = i as f32 / segments as f32;
        let u = 1.0 - t;

        let current_point =
            p0 * (u * u * u) + p1 * (3.0 * u * u * t) + p2 * (3.0 * u * t * t) + p3 * (t * t * t);

        gizmos.line_2d(prev_point, current_point, color);
        prev_point = current_point;
    }
}
