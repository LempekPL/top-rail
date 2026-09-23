use crate::camera::MainCamera;
use crate::controls::Controls;
use crate::state_manager::{DespawnWhenMainMenu, GameState, PlayingState};
use crate::util::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use std::cmp::PartialEq;

#[derive(Default)]
pub struct TrackPlugin;

// todo: better consts
const TRACK_SPACING: f32 = 20.0;
const NEW_TRACK_SNAP_RADIUS: f32 = 10.0;
const BUILD_SNAP_DISTANCE: f32 = 15.0;
const SEGMENT_LENGTH: f32 = 160.0;
const MIN_LENGTH_TO_BUILD: f32 = 20.0;
const MIN_RADIUS: f32 = 100.0;
const FINER_SEGMENT_RADIUS: f32 = 35.0;
// todo: make it more reasonable in the future
const MAX_DEPTH_SEARCH: usize = usize::MAX;
const _: () = assert!(
    NEW_TRACK_SNAP_RADIUS + TRACK_SPACING < FINER_SEGMENT_RADIUS,
    "track snapping needs to be bigger to use finer snapping"
);
const SIDE_CHANGE_RADIUS: f32 = 10.0;

#[derive(Resource, Default)]
struct TrackDebug {
    debug: bool,
}

fn update_debug_setting(
    controls: Controls,
    mut td: ResMut<TrackDebug>,
    mut s_text: Single<&mut Node, With<DebugTrackText>>,
) {
    if controls.just_pressed(|k| k.debug) {
        td.debug = !td.debug;
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

impl Plugin for TrackPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TrackBuilder>();
        app.insert_resource(TrackDebug { debug: false });
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
            ),
        );
        app.add_systems(OnExit(PlayingState::Build), reset_building);

        // debug systems
        app.add_systems(OnEnter(GameState::Playing), setup_debug_text);
        app.add_systems(
            Update,
            (debug_draw_track, debug_track_text).run_if(|td: Res<TrackDebug>| td.debug),
        );
        app.add_systems(Update, update_debug_setting);
    }
}

#[derive(Default, PartialEq, Clone)]
pub enum SnapNode {
    #[default]
    None,
    NoSnap {
        pos: Vec2,
    },
    DeadEnd {
        node: Entity,
        pos: Vec2,
        tangent: Vec2,
    },
    LoneNode {
        node: Entity,
        pos: Vec2,
        normal: Vec2,
    },
    Parallel {
        segment: Entity,
        pos: Vec2,
        normal: Vec2,
        side: i8,
    },
    NewJunction {
        segment: Entity,
        pos: Vec2,
        normal: Vec2,
    },
}

impl SnapNode {
    fn pos(&self) -> Vec2 {
        match self {
            SnapNode::None => Vec2::ZERO,
            SnapNode::NoSnap { pos, .. }
            | SnapNode::DeadEnd { pos, .. }
            | SnapNode::LoneNode { pos, .. }
            | SnapNode::Parallel { pos, .. }
            | SnapNode::NewJunction { pos, .. } => pos.clone(),
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
    pub p1: Vec2,
    pub p2: Vec2,
    pub start_node: Entity,
    pub end_node: Entity,
}

impl TrackSegment {
    pub fn new(p1: Vec2, p2: Vec2, start_node: Entity, end_node: Entity) -> Self {
        Self {
            p1,
            p2,
            start_node,
            end_node,
        }
    }

    pub fn bundle(p1: Vec2, p2: Vec2, start_node: Entity, end_node: Entity) -> impl Bundle {
        (Self::new(p1, p2, start_node, end_node), DespawnWhenMainMenu)
    }
}

#[derive(Component, Debug, Clone)]
pub enum TrackNode {
    DeadEnd {
        pos: Vec2,
        tangent: Vec2,
        track: Entity,
    },
    Continuation {
        pos: Vec2,
        normal: Vec2,
        tracks: [Entity; 2],
    },
    Junction {
        pos: Vec2,
        normal: Vec2,
        tracks: [Entity; 3],
        active_track_index: u8,
    },
    Crossing {
        pos: Vec2,
        normal: Vec2,
        tracks: [Entity; 4],
    },
}

impl TrackNode {
    pub fn bundle_end(pos: Vec2, tangent: Vec2, track: Entity) -> impl Bundle {
        (
            Self::DeadEnd {
                pos,
                tangent,
                track,
            },
            DespawnWhenMainMenu,
        )
    }

    pub fn bundle_cont(pos: Vec2, normal: Vec2, tracks: [Entity; 2]) -> impl Bundle {
        (
            Self::Continuation {
                pos,
                normal,
                tracks,
            },
            DespawnWhenMainMenu,
        )
    }

    pub fn bundle_junc(pos: Vec2, normal: Vec2, tracks: [Entity; 3]) -> impl Bundle {
        (
            Self::Junction {
                pos,
                normal,
                tracks,
                active_track_index: 0,
            },
            DespawnWhenMainMenu,
        )
    }

    pub fn add_track(&mut self, new_track: Entity) {
        match self {
            TrackNode::DeadEnd {
                pos,
                tangent,
                track,
            } => {
                let normal = Vec2::new(-tangent.y, tangent.x);
                *self = TrackNode::Continuation {
                    pos: *pos,
                    normal,
                    tracks: [*track, new_track],
                };
            }
            TrackNode::Continuation {
                pos,
                normal,
                tracks,
            } => {
                *self = TrackNode::Junction {
                    pos: *pos,
                    normal: *normal,
                    tracks: [tracks[0], tracks[1], new_track],
                    active_track_index: 0,
                };
            }
            TrackNode::Junction {
                pos,
                normal,
                tracks,
                ..
            } => {
                *self = TrackNode::Crossing {
                    pos: *pos,
                    normal: *normal,
                    tracks: [tracks[0], tracks[1], tracks[2], new_track],
                };
            }
            TrackNode::Crossing { .. } => {}
        }
    }

    pub fn remove_track(&mut self, old_track: Entity) {
        match self {
            TrackNode::DeadEnd { .. } => {}
            TrackNode::Continuation {
                pos,
                normal,
                tracks,
            } => {
                let tangent = Vec2::new(normal.y, -normal.x);
                let track = if tracks[0] == old_track {
                    tracks[1]
                } else {
                    tracks[0]
                };
                *self = TrackNode::DeadEnd {
                    pos: *pos,
                    tangent,
                    track,
                };
            }
            TrackNode::Junction {
                pos,
                normal,
                tracks,
                ..
            } => {
                let mut it = tracks.iter().copied().filter(|&t| t != old_track);
                *self = TrackNode::Continuation {
                    pos: *pos,
                    normal: *normal,
                    tracks: [it.next().unwrap(), it.next().unwrap()],
                };
            }
            TrackNode::Crossing {
                pos,
                normal,
                tracks,
            } => {
                let mut it = tracks.iter().copied().filter(|&t| t != old_track);
                *self = TrackNode::Junction {
                    pos: *pos,
                    normal: *normal,
                    tracks: [it.next().unwrap(), it.next().unwrap(), it.next().unwrap()],
                    active_track_index: 0,
                };
            }
        }
    }

    pub fn replace_track(&mut self, old_segment: Entity, new_segment: Entity) {
        match self {
            TrackNode::DeadEnd { track, .. } => {
                if *track == old_segment {
                    *track = new_segment;
                }
            }
            TrackNode::Continuation { tracks, .. } => {
                for track in tracks.iter_mut() {
                    if *track == old_segment {
                        *track = new_segment;
                    }
                }
            }
            TrackNode::Junction { tracks, .. } => {
                for track in tracks.iter_mut() {
                    if *track == old_segment {
                        *track = new_segment;
                    }
                }
            }
            TrackNode::Crossing { tracks, .. } => {
                for track in tracks.iter_mut() {
                    if *track == old_segment {
                        *track = new_segment;
                    }
                }
            }
        }
    }

    pub fn pos(&self) -> Vec2 {
        match self {
            TrackNode::DeadEnd { pos, .. }
            | TrackNode::Continuation { pos, .. }
            | TrackNode::Junction { pos, .. }
            | TrackNode::Crossing { pos, .. } => pos.clone(),
        }
    }
}

#[derive(SystemParam)]
pub struct Track<'w, 's> {
    pub segments: Query<'w, 's, (Entity, &'static TrackSegment)>,
    pub nodes: Query<'w, 's, (Entity, &'static TrackNode)>,
}

impl<'w, 's> Track<'w, 's> {
    pub fn iter_track(
        &self,
    ) -> impl Iterator<Item = (Entity, &TrackSegment, &TrackNode, &TrackNode)> {
        self.segments.iter().filter_map(move |(seg_ent, segment)| {
            let (_, start) = self.nodes.get(segment.start_node).ok()?;
            let (_, end) = self.nodes.get(segment.end_node).ok()?;
            Some((seg_ent, segment, start, end))
        })
    }

    pub fn iter_segments(&self) -> impl Iterator<Item = &TrackSegment> {
        self.segments.iter().map(move |(_, segment)| segment)
    }

    pub fn iter_nodes(&self) -> impl Iterator<Item = &TrackNode> {
        self.nodes.iter().map(move |(_, node)| node)
    }

    pub fn get_track(&self, seg_ent: Entity) -> Option<(&TrackSegment, &TrackNode, &TrackNode)> {
        self.segments.get(seg_ent).map_or(None, |(_, segment)| {
            let (_, start) = self.nodes.get(segment.start_node).ok()?;
            let (_, end) = self.nodes.get(segment.end_node).ok()?;
            Some((segment, start, end))
        })
    }
}

#[derive(SystemParam)]
pub struct TrackMut<'w, 's> {
    pub segments: Query<'w, 's, (Entity, &'static mut TrackSegment)>,
    pub nodes: Query<'w, 's, (Entity, &'static mut TrackNode)>,
}

impl<'w, 's> TrackMut<'w, 's> {
    pub fn as_readonly(&self) -> Track<'_, '_> {
        Track {
            segments: self.segments.as_readonly(),
            nodes: self.nodes.as_readonly(),
        }
    }

    pub fn get_track_mut(
        &mut self,
        seg_ent: Entity,
    ) -> Option<(
        Mut<'_, TrackSegment>,
        Mut<'_, TrackNode>,
        Mut<'_, TrackNode>,
    )> {
        let (_, segment) = self.segments.get_mut(seg_ent).ok()?;
        let start_ent = segment.start_node;
        let end_ent = segment.end_node;
        let [(_, start_node), (_, end_node)] =
            self.nodes.get_many_mut([start_ent, end_ent]).ok()?;

        Some((segment, start_node, end_node))
    }
}

fn build_track_snapper(
    mut gizmos: Gizmos,
    mut builder: ResMut<TrackBuilder>,
    s_window: Single<&Window, With<PrimaryWindow>>,
    s_camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    r_mouse: Res<ButtonInput<MouseButton>>,
    track: Track,
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

    for (entity, node) in track.nodes.iter() {
        if node.pos().distance(cursor_world_pos) < NEW_TRACK_SNAP_RADIUS {
            match node {
                TrackNode::DeadEnd { pos, tangent, .. } => {
                    snap = SnapNode::DeadEnd {
                        node: entity,
                        pos: *pos,
                        tangent: *tangent,
                    };
                }
                // todo: handle different nodes
                _ => {}
            }

            break;
        }
    }

    if matches!(snap, SnapNode::NoSnap { .. }) {
        let mut best_curve_point = None;
        let mut min_dist_to_curve = f32::MAX;

        for (entity, track_segment, start_node, end_node) in track.iter_track() {
            let mut is_close = false;
            let test_segments = 10;
            for i in 0..=test_segments {
                let t = i as f32 / test_segments as f32;
                let current_point = bezier::eval(
                    start_node.pos(),
                    track_segment.p1,
                    track_segment.p2,
                    end_node.pos(),
                    t,
                );
                if cursor_world_pos.distance(current_point) < FINER_SEGMENT_RADIUS {
                    is_close = true;
                    break;
                }
            }

            if is_close {
                let fine_segments = 100;
                for i in 0..=fine_segments {
                    let t = i as f32 / fine_segments as f32;
                    let current_point = bezier::eval(
                        start_node.pos(),
                        track_segment.p1,
                        track_segment.p2,
                        end_node.pos(),
                        t,
                    );

                    let dist = cursor_world_pos.distance(current_point);

                    if dist < min_dist_to_curve {
                        min_dist_to_curve = dist;
                        let tangent = bezier::derivative(
                            start_node.pos(),
                            track_segment.p1,
                            track_segment.p2,
                            end_node.pos(),
                            t,
                        )
                        .normalize_or_zero();

                        best_curve_point = Some((entity, current_point, tangent));
                    }
                }
            }
        }

        if let Some((entity, current_point, tangent)) = best_curve_point {
            if tangent != Vec2::ZERO {
                let normal = Vec2::new(-tangent.y, tangent.x);
                let snap_left = current_point + normal * TRACK_SPACING;
                let snap_right = current_point - normal * TRACK_SPACING;

                let dist_center = min_dist_to_curve;
                let dist_left = cursor_world_pos.distance(snap_left);
                let dist_right = cursor_world_pos.distance(snap_right);

                if dist_center < NEW_TRACK_SNAP_RADIUS {
                    snap = SnapNode::NewJunction {
                        pos: current_point,
                        segment: entity,
                        normal,
                    };
                } else if dist_left < NEW_TRACK_SNAP_RADIUS {
                    snap = SnapNode::Parallel {
                        pos: snap_left,
                        segment: entity,
                        normal,
                        side: 1,
                    };
                } else if dist_right < NEW_TRACK_SNAP_RADIUS {
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
    mut track: TrackMut,
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
    match builder.start_snap {
        SnapNode::DeadEnd { tangent, .. } => start_tangent = Some(tangent),
        SnapNode::NewJunction { normal, .. } | SnapNode::Parallel { normal, .. } => {
            let tangent = Vec2::new(normal.y, -normal.x);
            if p0.distance(p3) < SIDE_CHANGE_RADIUS {
                if (p0 - p3).dot(tangent) > 0.0 {
                    builder.drag_dir = 1;
                } else {
                    builder.drag_dir = -1;
                }
            }
            if builder.drag_dir == -1 {
                start_tangent = Some(tangent);
            } else {
                start_tangent = Some(-tangent);
            }
        }
        _ => {}
    }

    let mut segments = Vec::new();

    if let SnapNode::Parallel { normal, .. } | SnapNode::NewJunction { normal, .. } =
        builder.current_snap
    {
        let mut tangent = Vec2::new(normal.y, -normal.x);
        let chord = p3 - p0;
        let chord_dir = chord.normalize_or_zero();
        if chord.dot(tangent) < 0.0 {
            tangent = -tangent;
        }
        let t0 = start_tangent.unwrap_or(chord_dir);
        segments = create_segmented_bezier(p0, t0, p3, tangent, SEGMENT_LENGTH);
    } else if let SnapNode::DeadEnd { tangent, .. } = builder.current_snap {
        let chord_dir = (p3 - p0).normalize_or_zero();
        let t0 = start_tangent.unwrap_or(chord_dir);
        segments = create_segmented_bezier(p0, t0, p3, -tangent, SEGMENT_LENGTH);
    } else if let Some(tangent) = start_tangent {
        let normal = Vec2::new(-tangent.y, tangent.x);
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

    let clearance = TRACK_SPACING * 2.5;

    if matches!(builder.start_snap, SnapNode::NewJunction { .. }) && !segments.is_empty() {
        let (q0, q1, q2, q3) = segments[0];
        let dist = q0.distance(q3);
        if dist > clearance + MIN_LENGTH_TO_BUILD {
            let t = clearance / dist;
            let (s1, s2) = bezier::split_at_t(q0, q1, q2, q3, t);
            segments[0] = s2;
            segments.insert(0, s1);
        }
    }

    if matches!(builder.current_snap, SnapNode::NewJunction { .. }) && !segments.is_empty() {
        let last_idx = segments.len() - 1;
        let (q0, q1, q2, q3) = segments[last_idx];
        let dist = q0.distance(q3);
        if dist > clearance + MIN_LENGTH_TO_BUILD {
            let t = (dist - clearance) / dist;
            let (s1, s2) = bezier::split_at_t(q0, q1, q2, q3, t);
            segments[last_idx] = s1;
            segments.push(s2);
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

        let mut segment_entities = Vec::with_capacity(segments.len());
        for _ in 0..segments.len() {
            segment_entities.push(commands.spawn_empty().id());
        }

        let start_node_ent = if let SnapNode::NewJunction {
            segment: ent_seg,
            pos,
            normal,
        } = builder.start_snap
        {
            let Some((track_segment, mut start_n, mut end_n)) = track.get_track_mut(ent_seg) else {
                return;
            };

            if start_n.pos().distance(pos) < MIN_LENGTH_TO_BUILD {
                start_n.add_track(segment_entities[0]);
                track_segment.start_node
            } else if end_n.pos().distance(pos) < MIN_LENGTH_TO_BUILD {
                end_n.add_track(segment_entities[0]);
                track_segment.end_node
            } else {
                let (s1, s2) = bezier::split_at_pos(
                    start_n.pos(),
                    track_segment.p1,
                    track_segment.p2,
                    end_n.pos(),
                    pos,
                );
                commands.entity(ent_seg).despawn();

                let new_ts1 = commands.spawn_empty().id();
                start_n.replace_track(ent_seg, new_ts1);
                let new_ts2 = commands.spawn_empty().id();
                end_n.replace_track(ent_seg, new_ts2);

                let new_junction = commands
                    .spawn(TrackNode::bundle_junc(
                        pos,
                        normal,
                        [new_ts1, new_ts2, segment_entities[0]],
                    ))
                    .id();

                commands.entity(new_ts1).insert(TrackSegment::bundle(
                    s1.1,
                    s1.2,
                    track_segment.start_node,
                    new_junction,
                ));
                commands.entity(new_ts2).insert(TrackSegment::bundle(
                    s2.1,
                    s2.2,
                    new_junction,
                    track_segment.end_node,
                ));
                new_junction
            }
        } else if let SnapNode::DeadEnd { node, .. } = builder.start_snap {
            if let Ok((_, mut track_node)) = track.nodes.get_mut(node) {
                track_node.add_track(segment_entities[0]);
            }
            node
        } else {
            let (q0, q1, _, _) = segments[0];
            let dir = (q0 - q1).normalize_or_zero();
            commands
                .spawn(TrackNode::bundle_end(q0, dir, segment_entities[0]))
                .id()
        };

        let end_node_ent = if let SnapNode::NewJunction {
            segment: ent_seg,
            pos,
            normal,
        } = builder.current_snap
        {
            let Some((track_segment, mut start_n, mut end_n)) = track.get_track_mut(ent_seg) else {
                return;
            };

            if start_n.pos().distance(pos) < MIN_LENGTH_TO_BUILD {
                start_n.add_track(*segment_entities.last().unwrap());
                track_segment.start_node
            } else if end_n.pos().distance(pos) < MIN_LENGTH_TO_BUILD {
                end_n.add_track(*segment_entities.last().unwrap());
                track_segment.end_node
            } else {
                let (s1, s2) = bezier::split_at_pos(
                    start_n.pos(),
                    track_segment.p1,
                    track_segment.p2,
                    end_n.pos(),
                    pos,
                );
                commands.entity(ent_seg).despawn();

                let new_ts1 = commands.spawn_empty().id();
                start_n.replace_track(ent_seg, new_ts1);
                let new_ts2 = commands.spawn_empty().id();
                end_n.replace_track(ent_seg, new_ts2);

                let new_junction = commands
                    .spawn(TrackNode::bundle_junc(
                        pos,
                        normal,
                        [new_ts1, new_ts2, segment_entities[0]],
                    ))
                    .id();

                commands.entity(new_ts1).insert(TrackSegment::bundle(
                    s1.1,
                    s1.2,
                    track_segment.start_node,
                    new_junction,
                ));
                commands.entity(new_ts2).insert(TrackSegment::bundle(
                    s2.1,
                    s2.2,
                    new_junction,
                    track_segment.end_node,
                ));
                new_junction
            }
        } else if let SnapNode::DeadEnd { node, .. } = builder.current_snap {
            if let Ok((_, mut track_node)) = track.nodes.get_mut(node) {
                track_node.add_track(*segment_entities.last().unwrap());
            }
            node
        } else {
            let (_, _, q2, q3) = segments.last().unwrap();
            let dir = (q3 - q2).normalize_or_zero();
            commands
                .spawn(TrackNode::bundle_end(
                    *q3,
                    dir,
                    *segment_entities.last().unwrap(),
                ))
                .id()
        };

        let mut intermediate_nodes = Vec::with_capacity(segments.len().saturating_sub(1));
        for _ in 0..segments.len().saturating_sub(1) {
            intermediate_nodes.push(commands.spawn_empty().id());
        }

        for i in 0..segments.len() {
            let (_, q1, q2, q3) = segments[i];
            let is_last = i == segments.len() - 1;

            let seg_ent = segment_entities[i];
            let node_prev = if i == 0 {
                start_node_ent
            } else {
                intermediate_nodes[i - 1]
            };
            let node_next = if is_last {
                end_node_ent
            } else {
                intermediate_nodes[i]
            };

            commands
                .entity(seg_ent)
                .insert(TrackSegment::bundle(q1, q2, node_prev, node_next));

            if !is_last {
                let next_seg_ent = segment_entities[i + 1];
                let dir = (q3 - q2).normalize_or_zero();
                let normal = Vec2::new(-dir.y, dir.x);
                commands.entity(node_next).insert(TrackNode::bundle_cont(
                    q3,
                    normal,
                    [seg_ent, next_seg_ent],
                ));
            }
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
    s_camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    r_mouse: Res<ButtonInput<MouseButton>>,
    mut gizmos: Gizmos,
    mut track: TrackMut,
) {
    let (camera, camera_transform) = *s_camera;
    let Some(cursor_world_pos) = s_window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world_2d(camera_transform, cursor).ok())
    else {
        return;
    };

    let mut target_segment = None;
    let mut closest_dist = 15.0;
    const SAMPLES: usize = 10;
    for (seg_ent, segment, start_node, end_node) in track.as_readonly().iter_track() {
        for i in 0..=SAMPLES {
            let t = i as f32 / SAMPLES as f32;
            let point = bezier::eval(start_node.pos(), segment.p1, segment.p2, end_node.pos(), t);
            let dist = point.distance(cursor_world_pos);
            if dist < closest_dist {
                closest_dist = dist;
                target_segment = Some((
                    seg_ent,
                    segment.clone(),
                    start_node.clone(),
                    end_node.clone(),
                ));
            }
        }
    }

    let Some((deleted_seg_ent, deleted_seg, start_node, end_node)) = target_segment else {
        return;
    };

    // todo: replace with real red track in the future
    draw_bezier(
        &mut gizmos,
        start_node.pos(),
        deleted_seg.p1,
        deleted_seg.p2,
        end_node.pos(),
        Color::srgb(1., 0., 0.),
    );

    if !r_mouse.pressed(MouseButton::Left) {
        return;
    }
    let nodes_to_update = [deleted_seg.start_node, deleted_seg.end_node];
    commands.entity(deleted_seg_ent).despawn();
    for node_ent in nodes_to_update {
        let Ok((_, mut node)) = track.nodes.get_mut(node_ent) else {
            continue;
        };

        if let TrackNode::DeadEnd {
            track: old_track, ..
        } = *node
        {
            if old_track == deleted_seg_ent {
                commands.entity(node_ent).despawn();
                continue;
            }
        }

        node.remove_track(deleted_seg_ent);

        if let TrackNode::DeadEnd {
            track: surviving_ent,
            pos,
            ..
        } = *node
        {
            if let Ok((_, surviving_seg)) = track.segments.get(surviving_ent) {
                let tangent = if surviving_seg.start_node == node_ent {
                    (pos - surviving_seg.p1).normalize_or_zero()
                } else {
                    (pos - surviving_seg.p2).normalize_or_zero()
                };
                *node = TrackNode::DeadEnd {
                    pos,
                    tangent,
                    track: surviving_ent,
                };
            }
        }
    }
}

// fn find_closest_segment(
//     cursor_pos: Vec2,
//     q_segments: &Query<(Entity, &mut TrackSegment)>,
// ) -> Option<(Entity, f32, f32)> {
//     let mut min_dist = f32::MAX;
//     let mut best_match = None;
//
//     for (entity, segment) in q_segments.iter() {
//         for i in 0..=10 {
//             let t = i as f32 / 10.0;
//             let pos = bezier::eval(segment.p0, segment.p1, segment.p2, segment.p3, t);
//             let dist = pos.distance(cursor_pos);
//
//             if dist < min_dist {
//                 min_dist = dist;
//                 best_match = Some((entity, dist, t));
//             }
//         }
//     }
//
//     best_match
// }
//
// fn find_segment_path(
//     start: Entity,
//     end: Entity,
//     q_segments: &Query<(Entity, &mut TrackSegment)>,
// ) -> Option<Vec<Entity>> {
//     let mut queue = std::collections::VecDeque::new();
//     queue.push_back(vec![start]);
//     let mut visited = std::collections::HashSet::new();
//     visited.insert(start);
//
//     while let Some(path) = queue.pop_front() {
//         let curr = *path.last().unwrap();
//         if curr == end {
//             return Some(path);
//         }
//         if path.len() >= MAX_DEPTH_SEARCH {
//             continue;
//         }
//         if let Ok((_, seg)) = q_segments.get(curr) {
//             for conn in [&seg.start_node, &seg.end_node] {
//                 if let TrackConnection::Segment(next_ent) = conn {
//                     if !visited.contains(next_ent) {
//                         visited.insert(*next_ent);
//                         let mut new_path = path.clone();
//                         new_path.push(*next_ent);
//                         queue.push_back(new_path);
//                     }
//                 }
//             }
//         }
//     }
//     None
// }

// fn are_on_same_side(
//     path: &[Entity],
//     start_side: i8,
//     end_side: i8,
//     q_segments: &Query<(Entity, &mut TrackSegment)>,
// ) -> bool {
//     if path.is_empty() {
//         return start_side == end_side;
//     }
//
//     let mut current_orientation = 1;
//
//     for i in 0..(path.len() - 1) {
//         let curr_ent = path[i];
//         let next_ent = path[i + 1];
//
//         let Ok((_, curr_seg)) = q_segments.get(curr_ent) else {
//             continue;
//         };
//         let Ok((_, next_seg)) = q_segments.get(next_ent) else {
//             continue;
//         };
//
//         let exited_end = curr_seg.end_node == TrackConnection::Segment(next_ent);
//         let entered_end = next_seg.end_node == TrackConnection::Segment(curr_ent);
//
//         if exited_end == entered_end {
//             current_orientation *= -1;
//         }
//     }
//
//     (start_side * current_orientation) == end_side
// }

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
    track: Track,
    mut gizmos: Gizmos,
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
