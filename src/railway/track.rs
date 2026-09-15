use crate::camera;
use crate::railway::manager::{RailwayMode, RailwaySettings};
use crate::util::*;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

#[derive(Default)]
pub struct TrackPlugin;

const TRACK_SPACING: f32 = 20.0;
const SNAP_RADIUS: f32 = 10.0;
const SNAP_STRAIGHT_RADIUS: f32 = 15.0;
const SEGMENT_LENGTH: f32 = 100.0;
const MIN_LENGTH_TO_BUILD: f32 = 20.0;
const MIN_RADIUS: f32 = 100.0;

impl Plugin for TrackPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TrackBuilder>();
        app.add_systems(
            Update,
            (
                build_track_vis,
                build_track,
                bulldoze_track,
                debug_draw_track,
            ),
        );
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

impl TrackSegment {
    pub fn single(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2) -> Self {
        Self {
            p0,
            p1,
            p2,
            p3,
            start_node: TrackConnection::None,
            end_node: TrackConnection::None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackConnection {
    Node(Entity),
    LoneNode(Entity),
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

    pub fn new_transform(point: Vec2, dir: Vec2) -> (Self, Transform) {
        (
            Self::new((point - dir).normalize_or_zero()),
            Transform::from_translation(point.extend(0.)),
        )
    }
}

fn find_segment_path(
    start_ent: Entity,
    end_ent: Entity,
    drag_dir: Vec2,
    q_segments: &Query<(Entity, &mut TrackSegment)>,
) -> Option<Vec<(TrackSegment, bool)>> {
    if start_ent == end_ent {
        if let Ok((_, seg)) = q_segments.get(start_ent) {
            let tangent = (seg.p3 - seg.p0).normalize_or_zero();
            let is_forward = drag_dir.dot(tangent) >= 0.0;
            return Some(vec![(seg.clone(), is_forward)]);
        }
        return None;
    }

    let mut path = Vec::new();
    let mut current = start_ent;
    while current != end_ent {
        if let Ok((_, seg)) = q_segments.get(current) {
            path.push((seg.clone(), true));
            if let TrackConnection::Segment(next_ent) = seg.end_node {
                current = next_ent;
                if path.len() > 100 {
                    break;
                }
            } else {
                break;
            }
        } else {
            break;
        }
    }
    if current == end_ent {
        if let Ok((_, seg)) = q_segments.get(current) {
            path.push((seg.clone(), true));
            return Some(path);
        }
    }

    path.clear();
    current = start_ent;
    while current != end_ent {
        if let Ok((_, seg)) = q_segments.get(current) {
            path.push((seg.clone(), false));
            if let TrackConnection::Segment(prev_ent) = seg.start_node {
                current = prev_ent;
                if path.len() > 100 {
                    break;
                }
            } else {
                break;
            }
        } else {
            break;
        }
    }
    if current == end_ent {
        if let Ok((_, seg)) = q_segments.get(current) {
            path.push((seg.clone(), false));
            return Some(path);
        }
    }

    None
}

fn build_track_vis(
    mut gizmos: Gizmos,
    r_settings: Res<RailwaySettings>,
    s_window: Single<&Window, With<PrimaryWindow>>,
    s_camera: Single<(&Camera, &GlobalTransform), With<camera::MainCamera>>,
    q_nodes: Query<&GlobalTransform, With<TrackNode>>,
    mut q_segments: Query<(Entity, &mut TrackSegment)>,
) {
    if !matches!(r_settings.mode, RailwayMode::Build) {
        return;
    }
    let (camera, camera_transform) = *s_camera;
    let Some(cursor_world_pos) = s_window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world_2d(camera_transform, cursor).ok())
    else {
        return;
    };

    let mut pos = cursor_world_pos;
    for (transform) in q_nodes.iter() {
        let node_pos = transform.translation().truncate();
        if node_pos.distance(cursor_world_pos) < SNAP_RADIUS {
            pos = node_pos;
            break;
        }
    }

    gizmos.circle_2d(pos, 5., Color::WHITE);
}

fn build_track(
    mut commands: Commands,
    mut gizmos: Gizmos,
    mut builder: ResMut<TrackBuilder>,
    r_settings: Res<RailwaySettings>,
    s_window: Single<&Window, With<PrimaryWindow>>,
    s_camera: Single<(&Camera, &GlobalTransform), With<camera::MainCamera>>,
    r_mouse: Res<ButtonInput<MouseButton>>,
    q_nodes: Query<(Entity, &GlobalTransform, &TrackNode)>,
    mut q_segments: Query<(Entity, &mut TrackSegment)>,
) {
    if !matches!(r_settings.mode, RailwayMode::Build) {
        return;
    }
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
        let mut p3 = cursor_world_pos;

        let mut end_node = None;
        let mut end_tangent = None;
        for (entity, transform, node) in q_nodes.iter() {
            let node_pos = transform.translation().truncate();
            if node_pos.distance(cursor_world_pos) < SNAP_RADIUS {
                end_node = Some(entity);
                end_tangent = Some(node.outward_tangent);
                p3 = node_pos;
                break;
            }
        }

        let segments;
        if let Some(tangent) = end_tangent {
            let chord_dir = (p3 - p0).normalize_or_zero();
            let t0 = builder.start_tangent.unwrap_or(chord_dir);
            segments = create_segmented_bezier(p0, t0, p3, tangent, SEGMENT_LENGTH);
        } else if let Some(tangent) = builder.start_tangent {
            let normal = Vec2::new(-tangent.y, tangent.x);
            let chord = p3 - p0;
            let d = chord.dot(normal);
            let proj_dist = chord.dot(tangent);
            if proj_dist < 0. {
                // disallow curves >180deg
                p3 = p0 + normal * d;
            }
            if d.abs() < SNAP_STRAIGHT_RADIUS && proj_dist > 0. {
                segments = create_straight(p0, p0 + tangent * proj_dist, SEGMENT_LENGTH);
            } else {
                segments = create_arc(p0, tangent, p3, SEGMENT_LENGTH);
            }
        } else {
            segments = create_straight(p0, p3, SEGMENT_LENGTH);
        }

        let validation = validate_segments(&segments);

        if validation.drawable() {
            for (sg0, sg1, sg2, sg3) in segments.iter() {
                let color = if validation.spawnable() {
                    Color::srgb(0., 1., 0.)
                } else {
                    Color::srgb(1., 0., 0.)
                };
                draw_bezier(&mut gizmos, *sg0, *sg1, *sg2, *sg3, color);
                gizmos.circle_2d(*sg0, 3.0, Color::srgb(1., 1., 1.));
            }
        }

        if r_mouse.just_released(MouseButton::Left) {
            builder.is_dragging = false;
            if !validation.spawnable() {
                return;
            }

            let spawned_segments = segments
                .iter()
                .map(|_| commands.spawn_empty().id())
                .collect::<Vec<_>>();

            // update connections for old and new track
            let start_conn = if let Some(snapped_node) = builder.start_node {
                let mut snapped_track = None;
                for (ent, mut seg) in q_segments.iter_mut() {
                    if seg.start_node == TrackConnection::LoneNode(snapped_node) {
                        seg.start_node =
                            TrackConnection::Segment(*spawned_segments.first().unwrap());
                        snapped_track = Some(ent);
                        break;
                    } else if seg.end_node == TrackConnection::LoneNode(snapped_node) {
                        seg.end_node = TrackConnection::Segment(*spawned_segments.first().unwrap());
                        snapped_track = Some(ent);
                        break;
                    }
                }
                commands.entity(snapped_node).despawn();
                if let Some(snapped_track) = snapped_track {
                    TrackConnection::Segment(snapped_track)
                } else {
                    TrackConnection::None
                }
            } else {
                let first = segments.first().unwrap();
                TrackConnection::LoneNode(
                    commands
                        .spawn(TrackNode::new_transform(first.0, first.1))
                        .id(),
                )
            };

            let end_conn = if let Some(snapped_node) = end_node {
                let mut snapped_track = None;
                for (ent, mut seg) in q_segments.iter_mut() {
                    if seg.start_node == TrackConnection::LoneNode(snapped_node) {
                        seg.start_node =
                            TrackConnection::Segment(*spawned_segments.last().unwrap());
                        snapped_track = Some(ent);
                        break;
                    } else if seg.end_node == TrackConnection::LoneNode(snapped_node) {
                        seg.end_node = TrackConnection::Segment(*spawned_segments.last().unwrap());
                        snapped_track = Some(ent);
                        break;
                    }
                }
                commands.entity(snapped_node).despawn();
                if let Some(snapped_track) = snapped_track {
                    TrackConnection::Segment(snapped_track)
                } else {
                    TrackConnection::None
                }
            } else {
                let last = segments.last().unwrap();
                TrackConnection::LoneNode(
                    commands
                        .spawn(TrackNode::new_transform(last.3, last.2))
                        .id(),
                )
            };

            for i in 0..segments.len() {
                let current_ent = spawned_segments[i];

                let prev_conn = if i > 0 {
                    TrackConnection::Segment(spawned_segments[i - 1])
                } else {
                    start_conn.clone()
                };
                let next_conn = if i < spawned_segments.len() - 1 {
                    TrackConnection::Segment(spawned_segments[i + 1])
                } else {
                    end_conn.clone()
                };

                commands.entity(current_ent).insert(TrackSegment {
                    p0: segments[i].0,
                    p1: segments[i].1,
                    p2: segments[i].2,
                    p3: segments[i].3,
                    start_node: prev_conn,
                    end_node: next_conn,
                });
            }
        }
    }
}

fn bulldoze_track(
    mut commands: Commands,
    r_settings: Res<RailwaySettings>,
    s_window: Single<&Window, With<PrimaryWindow>>,
    s_camera: Single<(&Camera, &GlobalTransform), With<camera::MainCamera>>,
    r_mouse: Res<ButtonInput<MouseButton>>,
    mut q_segments: Query<(Entity, &mut TrackSegment)>,
) {
    if !matches!(r_settings.mode, RailwayMode::Bulldoze) {
        return;
    }
    if !r_mouse.pressed(MouseButton::Left) {
        return;
    }
    let (camera, camera_transform) = *s_camera;
    let Some(cursor_world_pos) = s_window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world_2d(camera_transform, cursor).ok())
    else {
        return;
    };

    let Some((target_ent, min_dist, _)) = find_closest_segment(cursor_world_pos, &q_segments)
    else {
        return;
    };

    if min_dist < 5.0 {
        let target_seg = if let Ok((_, seg)) = q_segments.get(target_ent) {
            seg.clone()
        } else {
            return;
        };

        if let TrackConnection::Segment(next_seg_ent) = target_seg.end_node {
            let out_tangent = (target_seg.p2 - target_seg.p3).normalize_or_zero();
            let new_node = commands
                .spawn((
                    TrackNode::new(out_tangent),
                    Transform::from_translation(target_seg.p3.extend(0.0)),
                ))
                .id();
            if let Ok((_, mut next_seg)) = q_segments.get_mut(next_seg_ent) {
                if next_seg.start_node == TrackConnection::Segment(target_ent) {
                    next_seg.start_node = TrackConnection::LoneNode(new_node);
                } else if next_seg.end_node == TrackConnection::Segment(target_ent) {
                    next_seg.end_node = TrackConnection::LoneNode(new_node);
                }
            }
        } else if let TrackConnection::LoneNode(node_ent) = target_seg.end_node {
            commands.entity(node_ent).despawn();
        }
        if let TrackConnection::Segment(prev_seg_ent) = target_seg.start_node {
            let out_tangent = (target_seg.p1 - target_seg.p0).normalize_or_zero();
            let new_node = commands
                .spawn((
                    TrackNode::new(out_tangent),
                    Transform::from_translation(target_seg.p0.extend(0.0)),
                ))
                .id();
            if let Ok((_, mut prev_seg)) = q_segments.get_mut(prev_seg_ent) {
                if prev_seg.start_node == TrackConnection::Segment(target_ent) {
                    prev_seg.start_node = TrackConnection::LoneNode(new_node);
                } else if prev_seg.end_node == TrackConnection::Segment(target_ent) {
                    prev_seg.end_node = TrackConnection::LoneNode(new_node);
                }
            }
        } else if let TrackConnection::LoneNode(node_ent) = target_seg.start_node {
            commands.entity(node_ent).despawn();
        }
        commands.entity(target_ent).despawn();
    }
}

fn find_closest_segment(
    cursor_pos: Vec2,
    q_segments: &Query<(Entity, &mut TrackSegment)>,
) -> Option<(Entity, f32, f32)> {
    let mut min_dist = f32::MAX;
    let mut best_match = None;

    for (entity, segment) in q_segments.iter() {
        for i in 0..=10 {
            let t = i as f32 / 10.0;
            let pos = bezier::eval(segment.p0, segment.p1, segment.p2, segment.p3, t);
            let dist = pos.distance(cursor_pos);

            if dist < min_dist {
                min_dist = dist;
                best_match = Some((entity, dist, t));
            }
        }
    }

    best_match
}

enum ValidatedTrack {
    Valid,
    SegmentSharp,
    Empty,
    TooShort,
}

impl ValidatedTrack {
    fn drawable(&self) -> bool {
        self.spawnable() || matches!(self, ValidatedTrack::SegmentSharp)
    }

    fn spawnable(&self) -> bool {
        matches!(self, ValidatedTrack::Valid)
    }
}
fn validate_segments(segments: &Vec<(Vec2, Vec2, Vec2, Vec2)>) -> ValidatedTrack {
    if segments.is_empty() {
        return ValidatedTrack::Empty;
    }
    let first = segments.first().unwrap();
    let last = segments.last().unwrap();
    if first.0.distance(last.3) < MIN_LENGTH_TO_BUILD && first == last {
        return ValidatedTrack::TooShort;
    }
    for (s1, s2, s3, s4) in segments {
        if !is_segment_valid(*s1, *s2, *s3, *s3, MIN_RADIUS) {
            return ValidatedTrack::SegmentSharp;
        }
    }
    ValidatedTrack::Valid
}

pub fn is_segment_valid(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, min_radius: f32) -> bool {
    for i in 0..=10 {
        let t = i as f32 / 10.0;
        let d1 = bezier::derivative(p0, p1, p2, p3, t);
        let d2 = bezier::second_derivative(p0, p1, p2, p3, t);
        let cross = (d1.x * d2.y - d1.y * d2.x).abs();
        let denom = d1.length_squared().powf(1.5);
        if denom > 0.0001 {
            let curvature = cross / denom;
            if curvature > 0.0 {
                let radius = 1.0 / curvature;
                if radius < min_radius {
                    return false;
                }
            }
        }
    }
    true
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
        gizmos.circle_2d(segment.p0, 3.0, Color::srgb(1.0, 1.0, 1.0));
        gizmos.circle_2d(segment.p3, 3.0, Color::srgb(1.0, 1.0, 1.0));
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
