use crate::camera;
use crate::state_manager::{DespawnWhenMainMenu, PlayingState};
use crate::util::*;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use std::cmp::PartialEq;

#[derive(Default)]
pub struct TrackPlugin;

const TRACK_SPACING: f32 = 20.0;
const NEW_TRACK_SNAP_RADIUS: f32 = 10.0;
const BUILD_SNAP_DISTANCE: f32 = 15.0;
const SEGMENT_LENGTH: f32 = 100.0;
const MIN_LENGTH_TO_BUILD: f32 = 20.0;
const MIN_RADIUS: f32 = 100.0;
const FINER_SEGMENT_RADIUS: f32 = 35.0;
// todo: make it more reasonable in the future
const MAX_DEPTH_SEARCH: usize = usize::MAX;
const _: () = assert!(
    NEW_TRACK_SNAP_RADIUS + TRACK_SPACING < FINER_SEGMENT_RADIUS,
    "track snapping needs to be bigger to use finer snapping"
);

impl Plugin for TrackPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TrackBuilder>();
        app.add_systems(
            Update,
            (
                (
                    build_track_snapper,
                    build_track_spawner.run_if(|builder: Res<TrackBuilder>| builder.is_building),
                )
                    .chain()
                    .run_if(in_state(PlayingState::Build)),
                bulldoze_track.run_if(in_state(PlayingState::Bulldoze)),
                debug_draw_track,
            ),
        );
        app.add_systems(OnExit(PlayingState::Build), reset_building);
    }
}

#[derive(Default, PartialEq, Clone)]
pub enum SnapNode {
    #[default]
    None,
    NoSnap {
        pos: Vec2,
    },
    LoneNode {
        node: Entity,
        pos: Vec2,
        tangent: Vec2,
    },
    Parallel {
        segment: Entity,
        pos: Vec2,
        normal: Vec2,
        side: i8,
    },
    Junction {
        segment: Entity,
        pos: Vec2,
        normal: Vec2,
    },
}

impl SnapNode {
    pub fn tangent(&self) -> Option<Vec2> {
        match self {
            SnapNode::LoneNode { tangent, .. }
            | SnapNode::Parallel {
                normal: tangent, ..
            }
            | SnapNode::Junction {
                normal: tangent, ..
            } => Some(*tangent),
            _ => None,
        }
    }

    fn pos(&self) -> Vec2 {
        match self {
            SnapNode::None => Vec2::ZERO,
            SnapNode::NoSnap { pos, .. }
            | SnapNode::LoneNode { pos, .. }
            | SnapNode::Parallel { pos, .. }
            | SnapNode::Junction { pos, .. } => pos.clone(),
        }
    }
}

#[derive(Resource, Default)]
pub struct TrackBuilder {
    pub is_building: bool,
    pub drag_dir: i8,
    pub start_snap: SnapNode,
    pub current_snap: SnapNode,
}

impl TrackBuilder {
    fn stop(&mut self) {
        self.is_building = false;
        self.drag_dir = 0;
    }

    fn reset(&mut self) {
        self.is_building = false;
        self.drag_dir = 0;
        self.current_snap = SnapNode::None;
        self.start_snap = SnapNode::None;
    }
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

    pub fn new_transform(point: Vec2, dir: Vec2) -> impl Bundle {
        (
            Self::new((point - dir).normalize_or_zero()),
            Transform::from_translation(point.extend(0.)),
            DespawnWhenMainMenu,
        )
    }
}

fn build_track_snapper(
    mut gizmos: Gizmos,
    mut builder: ResMut<TrackBuilder>,
    s_window: Single<&Window, With<PrimaryWindow>>,
    s_camera: Single<(&Camera, &GlobalTransform), With<camera::MainCamera>>,
    r_mouse: Res<ButtonInput<MouseButton>>,
    q_nodes: Query<(Entity, &GlobalTransform, &TrackNode)>,
    q_segments: Query<(Entity, &TrackSegment)>,
) {
    let (camera, camera_transform) = *s_camera;
    let Some(cursor_world_pos) = s_window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world_2d(camera_transform, cursor).ok())
    else {
        return;
    };

    let mut snap = SnapNode::NoSnap {
        pos: cursor_world_pos,
    };

    for (entity, transform, node) in q_nodes.iter() {
        let node_pos = transform.translation().truncate();
        if node_pos.distance(cursor_world_pos) < NEW_TRACK_SNAP_RADIUS {
            snap = SnapNode::LoneNode {
                pos: node_pos,
                node: entity,
                tangent: node.outward_tangent,
            };
            break;
        }
    }

    if matches!(snap, SnapNode::NoSnap { .. }) {
        let mut min_snap_dist = NEW_TRACK_SNAP_RADIUS;
        for (entity, ts) in q_segments.iter() {
            let mut is_close = false;
            let test_segments = 10;
            for i in 0..=test_segments {
                let t = i as f32 / test_segments as f32;
                let current_point = bezier::eval(ts.p0, ts.p1, ts.p2, ts.p3, t);
                if cursor_world_pos.distance(current_point) < FINER_SEGMENT_RADIUS {
                    is_close = true;
                    break;
                }
            }
            if is_close {
                let fine_segments = 100;
                for i in 0..=fine_segments {
                    let t = i as f32 / fine_segments as f32;
                    let current_point = bezier::eval(ts.p0, ts.p1, ts.p2, ts.p3, t);
                    let tangent =
                        bezier::derivative(ts.p0, ts.p1, ts.p2, ts.p3, t).normalize_or_zero();
                    if tangent == Vec2::ZERO {
                        continue;
                    }
                    let normal = Vec2::new(-tangent.y, tangent.x);
                    let dist_center = cursor_world_pos.distance(current_point);
                    if dist_center < min_snap_dist {
                        min_snap_dist = dist_center;
                        snap = SnapNode::Junction {
                            pos: current_point,
                            segment: entity,
                            normal,
                        };
                    }
                    let snap_left = current_point + normal * TRACK_SPACING;
                    let dist_left = cursor_world_pos.distance(snap_left);
                    if dist_left < min_snap_dist {
                        min_snap_dist = dist_left;
                        snap = SnapNode::Parallel {
                            pos: snap_left,
                            segment: entity,
                            normal,
                            side: 1,
                        };
                    }
                    let snap_right = current_point - normal * TRACK_SPACING;
                    let dist_right = cursor_world_pos.distance(snap_right);
                    if dist_right < min_snap_dist {
                        min_snap_dist = dist_right;
                        snap = SnapNode::Parallel {
                            pos: snap_right,
                            segment: entity,
                            normal,
                            side: -1,
                        };
                    }
                }
            }
        }
    }
    gizmos.circle_2d(snap.pos(), 5., Color::WHITE);
    if r_mouse.just_pressed(MouseButton::Left) {
        builder.is_building = true;
        builder.start_snap = snap;
    } else if builder.is_building {
        builder.current_snap = snap;
    }
}

fn build_track_spawner(
    mut commands: Commands,
    mut gizmos: Gizmos,
    mut builder: ResMut<TrackBuilder>,
    r_mouse: Res<ButtonInput<MouseButton>>,
    q_nodes: Query<(Entity, &GlobalTransform, &TrackNode)>,
    mut q_segments: Query<(Entity, &mut TrackSegment)>,
) {
    if matches!(builder.current_snap, SnapNode::None) {
        return;
    }
    let p0 = builder.start_snap.pos();
    let mut p3 = builder.current_snap.pos();
    if p0 == p3 {
        return;
    }
    let mut start_tangent = None;
    let mut start_normal = None;
    match builder.start_snap {
        SnapNode::LoneNode { tangent, .. } => {
            start_tangent = Some(tangent);
            start_normal = Some(Vec2::new(-tangent.y, tangent.x));
        }
        SnapNode::Parallel { normal, .. } | SnapNode::Junction { normal, .. } => {
            start_normal = Some(normal);
            let chord = p3 - p0;
            if chord.length_squared() < MIN_LENGTH_TO_BUILD {
                let track_dir_candidate = Vec2::new(normal.y, -normal.x);
                if chord.dot(track_dir_candidate) > 0. {
                    builder.drag_dir = 1;
                } else {
                    builder.drag_dir = -1;
                }
            }
            if builder.drag_dir != 0 {
                start_tangent = Some(Vec2::new(normal.y, -normal.x) * builder.drag_dir as f32);
            } else {
                start_tangent = Some(Vec2::new(normal.y, -normal.x));
            }
        }
        _ => {}
    }
    let mut segments = Vec::new();
    let mut parallel_success = false;

    if let (
        SnapNode::Parallel {
            segment: start_segment,
            side: start_side,
            normal: start_normal,
            pos: start_pos,
        },
        SnapNode::Parallel {
            segment: end_segment,
            side: end_side,
            normal: end_normal,
            pos: end_pos,
        },
    ) = (builder.start_snap.clone(), builder.current_snap.clone())
    {
        if let Some(path) = find_segment_path(start_segment, end_segment, &q_segments)
            && are_on_same_side(&path, start_side, end_side, &q_segments)
        {
            
        }
    }

    if !parallel_success {
        if let SnapNode::Parallel { normal, .. } | SnapNode::Junction { normal, .. } =
            builder.current_snap
        {
            let chord_dir = (p3 - p0).normalize_or_zero();
            let mut track_dir_candidate = Vec2::new(normal.y, -normal.x);
            if track_dir_candidate.dot(chord_dir) < 0. {
                track_dir_candidate = -track_dir_candidate;
            }
            let t0 = start_tangent.unwrap_or(chord_dir);
            let mut tmp_seg =
                create_segmented_bezier(p0, t0, p3, track_dir_candidate, SEGMENT_LENGTH);
            if matches!(validate_segments(&tmp_seg), ValidatedTrack::SegmentSharp) {
                let tmp_seg2 =
                    create_segmented_bezier(p0, t0, p3, -track_dir_candidate, SEGMENT_LENGTH);
                if !matches!(validate_segments(&tmp_seg2), ValidatedTrack::SegmentSharp) {
                    tmp_seg = tmp_seg2;
                }
            }
            segments = tmp_seg;
        } else if let SnapNode::LoneNode { tangent, .. } = builder.current_snap {
            let chord_dir = (p3 - p0).normalize_or_zero();
            let t0 = start_tangent.unwrap_or(chord_dir);
            segments = create_segmented_bezier(p0, t0, p3, -tangent, SEGMENT_LENGTH);
        } else if let Some(tangent) = start_tangent {
            let normal = start_normal.unwrap();
            let chord = p3 - p0;
            let d = chord.dot(normal);
            let proj_dist = chord.dot(tangent);
            if proj_dist < 0. {
                // disallow curves >180deg
                p3 = p0 + normal * d;
            }
            let dist_straight = d.abs();
            let dist_45_pos = ((d - proj_dist) * std::f32::consts::FRAC_1_SQRT_2).abs();
            let dist_45_neg = ((d + proj_dist) * std::f32::consts::FRAC_1_SQRT_2).abs();
            if dist_straight < BUILD_SNAP_DISTANCE && proj_dist > 0. {
                segments = create_straight(p0, p0 + tangent * proj_dist, SEGMENT_LENGTH);
            } else if dist_45_pos < BUILD_SNAP_DISTANCE && proj_dist > 0. {
                let snap_len = (proj_dist + d) * std::f32::consts::FRAC_1_SQRT_2;
                let snap_dir = (tangent + normal).normalize();
                segments = create_arc(p0, tangent, p0 + snap_dir * snap_len, SEGMENT_LENGTH);
            } else if dist_45_neg < BUILD_SNAP_DISTANCE && proj_dist > 0. {
                let snap_len = (proj_dist - d) * std::f32::consts::FRAC_1_SQRT_2;
                let snap_dir = (tangent - normal).normalize();
                segments = create_arc(p0, tangent, p0 + snap_dir * snap_len, SEGMENT_LENGTH);
            } else {
                segments = create_arc(p0, tangent, p3, SEGMENT_LENGTH);
            }
        } else {
            segments = create_straight(p0, p3, SEGMENT_LENGTH);
        }
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

    if !r_mouse.pressed(MouseButton::Left) {
        if !validation.spawnable() {
            builder.reset();
            return;
        }

        let spawned_segments = segments
            .iter()
            .map(|_| commands.spawn(DespawnWhenMainMenu).id())
            .collect::<Vec<_>>();

        // update connections for old and new track
        let start_conn = if let SnapNode::LoneNode {
            node: snapped_node, ..
        } = builder.start_snap
        {
            let mut snapped_track = None;
            for (ent, mut seg) in q_segments.iter_mut() {
                if seg.start_node == TrackConnection::LoneNode(snapped_node) {
                    seg.start_node = TrackConnection::Segment(*spawned_segments.first().unwrap());
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

        let end_conn = if let SnapNode::LoneNode {
            node: snapped_node, ..
        } = builder.current_snap
        {
            let mut snapped_track = None;
            for (ent, mut seg) in q_segments.iter_mut() {
                if seg.start_node == TrackConnection::LoneNode(snapped_node) {
                    seg.start_node = TrackConnection::Segment(*spawned_segments.last().unwrap());
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
        builder.reset();
    }
}

fn reset_building(mut builder: ResMut<TrackBuilder>) {
    builder.stop();
}

fn bulldoze_track(
    mut commands: Commands,
    s_window: Single<&Window, With<PrimaryWindow>>,
    s_camera: Single<(&Camera, &GlobalTransform), With<camera::MainCamera>>,
    r_mouse: Res<ButtonInput<MouseButton>>,
    mut q_segments: Query<(Entity, &mut TrackSegment)>,
    q_interactions: Query<&Interaction, With<Node>>,
) {
    if !r_mouse.pressed(MouseButton::Left) {
        return;
    }
    if q_interactions
        .iter()
        .any(|interaction| *interaction != Interaction::None)
    {
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
            let new_node = commands
                .spawn((
                    TrackNode::new((target_seg.p2 - target_seg.p3).normalize_or_zero()),
                    Transform::from_translation(target_seg.p3.extend(0.)),
                    DespawnWhenMainMenu,
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
            let new_node = commands
                .spawn((
                    TrackNode::new((target_seg.p1 - target_seg.p0).normalize_or_zero()),
                    Transform::from_translation(target_seg.p0.extend(0.)),
                    DespawnWhenMainMenu,
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

fn find_segment_path(
    start: Entity,
    end: Entity,
    q_segments: &Query<(Entity, &mut TrackSegment)>,
) -> Option<Vec<Entity>> {
    let mut queue = std::collections::VecDeque::new();
    queue.push_back(vec![start]);
    let mut visited = std::collections::HashSet::new();
    visited.insert(start);

    while let Some(path) = queue.pop_front() {
        let curr = *path.last().unwrap();
        if curr == end {
            return Some(path);
        }
        if path.len() >= MAX_DEPTH_SEARCH {
            continue;
        }
        if let Ok((_, seg)) = q_segments.get(curr) {
            for conn in [&seg.start_node, &seg.end_node] {
                if let TrackConnection::Segment(next_ent) = conn {
                    if !visited.contains(next_ent) {
                        visited.insert(*next_ent);
                        let mut new_path = path.clone();
                        new_path.push(*next_ent);
                        queue.push_back(new_path);
                    }
                }
            }
        }
    }
    None
}

fn are_on_same_side(
    path: &[Entity],
    start_side: i8,
    end_side: i8,
    q_segments: &Query<(Entity, &mut TrackSegment)>,
) -> bool {
    if path.is_empty() {
        return start_side == end_side;
    }

    let mut current_orientation = 1;

    for i in 0..(path.len() - 1) {
        let curr_ent = path[i];
        let next_ent = path[i + 1];

        let Ok((_, curr_seg)) = q_segments.get(curr_ent) else {
            continue;
        };
        let Ok((_, next_seg)) = q_segments.get(next_ent) else {
            continue;
        };

        let exited_end = curr_seg.end_node == TrackConnection::Segment(next_ent);
        let entered_end = next_seg.end_node == TrackConnection::Segment(curr_ent);

        if exited_end == entered_end {
            current_orientation *= -1;
        }
    }

    (start_side * current_orientation) == end_side
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
    for (s0, s1, s2, s3) in segments {
        if !is_segment_valid(*s0, *s1, *s2, *s3, MIN_RADIUS) {
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
            NEW_TRACK_SNAP_RADIUS,
            Color::srgb(0.2, 0.5, 1.0),
        );

        gizmos.arrow_2d(
            transform.translation().xy(),
            transform.translation().xy() + node.outward_tangent * 10.0,
            Color::srgb(1.0, 1.0, 0.2),
        );
    }
}
