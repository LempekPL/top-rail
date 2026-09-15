use crate::camera;
use crate::railway::manager::{RailwayMode, RailwaySettings};
use crate::util::*;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

#[derive(Default)]
pub struct TrackPlugin;

const TRACK_SPACING: f32 = 20.0;
const SNAP_RADIUS: f32 = 10.0;
const LINE_SEGMENT_LENGTH: f32 = 100.0;
const MIN_RADIUS: f32 = 100.0;

impl Plugin for TrackPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TrackBuilder>();
        app.add_systems(Update, (build_track, bulldoze_track, debug_draw_track));
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
            if let Some((entity, dist, t)) = find_closest_segment(cursor_world_pos, &q_segments) {
                if dist < TRACK_SPACING + 20.0 {
                    if let Ok((_, segment)) = q_segments.get(entity) {
                        let pos = eval_bezier(segment.p0, segment.p1, segment.p2, segment.p3, t);
                        let tan =
                            eval_derivative(segment.p0, segment.p1, segment.p2, segment.p3, t)
                                .normalize_or_zero();
                        let normal = Vec2::new(-tan.y, tan.x);
                        let side = (cursor_world_pos - pos).dot(normal).signum();

                        builder.start_pos = pos + normal * TRACK_SPACING * side;
                        builder.start_tangent = Some(tan);
                        builder.start_node = None;
                        snapped = true;
                    }
                }
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
        let mut p0 = builder.start_pos;
        let mut p3 = cursor_world_pos;

        let mut snapped_end_node = None;
        let mut snapped_end_tangent = None;
        let mut split_segments = Vec::new();
        let mut is_parallel_build = false;

        for (entity, transform, node) in q_nodes.iter() {
            if Some(entity) == builder.start_node {
                continue;
            }
            let node_pos = transform.translation().truncate();
            if node_pos.distance(cursor_world_pos) < SNAP_RADIUS {
                p3 = node_pos;
                snapped_end_tangent = Some(node.outward_tangent);
                snapped_end_node = Some(entity);
                break;
            }
        }

        if snapped_end_node.is_none() {
            if let Some((entity, dist, t)) = find_closest_segment(cursor_world_pos, &q_segments) {
                if dist < TRACK_SPACING + 20.0 {
                    if let Ok((_, segment)) = q_segments.get(entity) {
                        let pos = eval_bezier(segment.p0, segment.p1, segment.p2, segment.p3, t);
                        let tan =
                            eval_derivative(segment.p0, segment.p1, segment.p2, segment.p3, t)
                                .normalize_or_zero();
                        let normal = Vec2::new(-tan.y, tan.x);
                        let side = (cursor_world_pos - pos).dot(normal).signum();

                        p3 = pos + normal * TRACK_SPACING * side;

                        let chord_dir_temp = (p3 - p0).normalize_or_zero();
                        let mut final_tan = tan;
                        if final_tan.dot(chord_dir_temp) < 0.0 {
                            final_tan = -final_tan;
                        }
                        snapped_end_tangent = Some(final_tan);
                    }
                }
            }
        }

        if snapped_end_node.is_none() {
            if let Some((start_ent, d_start, start_t)) = find_closest_segment(p0, &q_segments) {
                if let Some((end_ent, d_end, end_t)) = find_closest_segment(p3, &q_segments) {
                    // Jeśli oba końce leżą idealnie na odpowiednim dystansie od torów
                    if (d_start - TRACK_SPACING).abs() < 5.0 && (d_end - TRACK_SPACING).abs() < 5.0
                    {
                        let drag_dir = p3 - p0;
                        if let Some(path) =
                            find_segment_path(start_ent, end_ent, drag_dir, &q_segments)
                        {
                            is_parallel_build = true;

                            // Oblicz dokładny offset ze startowego segmentu
                            let Ok((_, start_seg)) = q_segments.get(start_ent) else {
                                return;
                            };
                            let proj_p0 = eval_bezier(
                                start_seg.p0,
                                start_seg.p1,
                                start_seg.p2,
                                start_seg.p3,
                                start_t,
                            );
                            let tan_p0 = eval_derivative(
                                start_seg.p0,
                                start_seg.p1,
                                start_seg.p2,
                                start_seg.p3,
                                start_t,
                            )
                            .normalize_or_zero();
                            let normal_p0 = Vec2::new(-tan_p0.y, tan_p0.x);
                            let side = (p0 - proj_p0).dot(normal_p0).signum();
                            let exact_offset = TRACK_SPACING * side;

                            for (i, (seg, is_forward)) in path.iter().enumerate() {
                                // Ustalamy granice ucinania (tylko pierwszy i ostatni segment są ucinane)
                                let local_start = if i == 0 {
                                    start_t
                                } else {
                                    if *is_forward { 0.0 } else { 1.0 }
                                };
                                let local_end = if i == path.len() - 1 {
                                    end_t
                                } else {
                                    if *is_forward { 1.0 } else { 0.0 }
                                };

                                let actual_is_forward = local_start <= local_end;
                                let t_min = local_start.min(local_end);
                                let t_max = local_start.max(local_end);

                                // Ucinamy i natychmiast odsuwamy na bok
                                let (sq0, sq1, sq2, sq3) = extract_sub_bezier(
                                    seg.p0, seg.p1, seg.p2, seg.p3, t_min, t_max,
                                );
                                split_segments.push(offset_bezier(
                                    sq0,
                                    sq1,
                                    sq2,
                                    sq3,
                                    exact_offset,
                                    !actual_is_forward,
                                ));
                            }

                            if let Some(first) = split_segments.first() {
                                p0 = first.0;
                            }
                            if let Some(last) = split_segments.last() {
                                p3 = last.3;
                            }
                        }
                    }
                }
            }
        }

        if !is_parallel_build {
            let chord = p3 - p0;
            let mut dist = chord.length();
            let chord_dir = chord.normalize_or_zero();

            let mut straight_line_dir = None;
            let actual_t0 = builder.start_tangent;

            if let Some(t3_out) = snapped_end_tangent {
                let t0 = actual_t0.unwrap_or(chord_dir);
                let t3_in = -t3_out;

                let dot_ends = t0.dot(t3_in);
                let dot_0_chord = t0.dot(chord_dir);
                let dot_3_chord = t3_in.dot(chord_dir);

                // todo: fix too straight line
                if dot_0_chord > 0.99 && dot_3_chord > 0.99 {
                    straight_line_dir = Some(chord_dir);
                } else if (dot_0_chord > 0.85 && dot_3_chord > 0.85)
                    || (dot_ends > 0.85 && dot_0_chord > 0.2 && dot_3_chord > 0.2)
                {
                    let is_s_curve = dot_ends > 0.85 && dot_0_chord <= 0.95;
                    let d = dist * if is_s_curve { 0.45 } else { 0.33 };

                    let p1 = p0 + t0 * d;
                    let p2 = p3 - t3_in * d;

                    let num_splits = (dist / LINE_SEGMENT_LENGTH).ceil().max(1.0) as usize;
                    let step = 1.0 / num_splits as f32;

                    for i in 0..num_splits {
                        let ta = i as f32 * step;
                        let tb = (i + 1) as f32 * step;

                        let q0 = eval_bezier(p0, p1, p2, p3, ta);
                        let q3 = eval_bezier(p0, p1, p2, p3, tb);

                        let q1 = q0 + eval_derivative(p0, p1, p2, p3, ta) * (step / 3.0);
                        let q2 = q3 - eval_derivative(p0, p1, p2, p3, tb) * (step / 3.0);

                        split_segments.push((q0, q1, q2, q3));
                    }
                } else {
                    split_segments =
                        calculate_path(p0, t0, p3, t3_in, MIN_RADIUS, LINE_SEGMENT_LENGTH);
                }
            } else if let Some(t0) = builder.start_tangent {
                let n0 = Vec2::new(-t0.y, t0.x);
                let d = chord.dot(n0);
                if d.abs() > 15.0 {
                    let turn_dir = d.signum();
                    let mut radius = (chord.length_squared() / (2.0 * d.abs())).abs();
                    radius = radius.max(MIN_RADIUS);
                    let center = p0 + n0 * radius * turn_dir;
                    split_segments =
                        generate_arc(center, radius, p0, p3, d < 0.0, LINE_SEGMENT_LENGTH);
                } else {
                    // Dociągamy kursor fizycznie do idealnej osi prostej!
                    let mut proj_dist = chord.dot(t0);
                    let mut actual_dir = t0;

                    if proj_dist < 0.0 {
                        actual_dir = -t0;
                        proj_dist = -proj_dist;
                    }

                    straight_line_dir = Some(actual_dir);
                    p3 = p0 + actual_dir * proj_dist;
                    dist = proj_dist;
                }
            } else {
                straight_line_dir = Some(chord_dir);
            }

            if let Some(straight_line_dir) = straight_line_dir {
                split_segments =
                    generate_straight(straight_line_dir, dist, p0, LINE_SEGMENT_LENGTH);
            }
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
            if p0.distance(p3) < SNAP_RADIUS {
                return;
            }
            let first = split_segments.first().unwrap();
            let last = split_segments.last().unwrap();
            let mut start_conn = TrackConnection::None;
            if let Some(snapped_node_ent) = builder.start_node {
                let mut old_track_ent = None;
                for (seg_ent, seg) in q_segments.iter() {
                    if seg.start_node == TrackConnection::LoneNode(snapped_node_ent)
                        || seg.end_node == TrackConnection::LoneNode(snapped_node_ent)
                    {
                        old_track_ent = Some(seg_ent);
                        break;
                    }
                }
                commands.entity(snapped_node_ent).despawn();
                if let Some(old_seg) = old_track_ent {
                    start_conn = TrackConnection::Segment(old_seg);
                }
            } else {
                let new_node = commands
                    .spawn((
                        TrackNode::new((p0 - first.1).normalize_or_zero()),
                        Transform::from_translation(p0.extend(0.)),
                    ))
                    .id();
                start_conn = TrackConnection::LoneNode(new_node);
            }
            let mut end_conn = TrackConnection::None;
            if let Some(snapped_node_ent) = snapped_end_node {
                let mut old_track_ent = None;
                for (seg_ent, seg) in q_segments.iter() {
                    if seg.start_node == TrackConnection::LoneNode(snapped_node_ent)
                        || seg.end_node == TrackConnection::LoneNode(snapped_node_ent)
                    {
                        old_track_ent = Some(seg_ent);
                        break;
                    }
                }
                commands.entity(snapped_node_ent).despawn();
                if let Some(old_seg) = old_track_ent {
                    end_conn = TrackConnection::Segment(old_seg);
                }
            } else {
                let new_node = commands
                    .spawn((
                        TrackNode::new((last.3 - last.2).normalize_or_zero()),
                        Transform::from_translation(last.3.extend(0.)),
                    ))
                    .id();
                end_conn = TrackConnection::LoneNode(new_node);
            }

            let mut spawned_segments = Vec::new();
            for (q0, q1, q2, q3) in split_segments.iter() {
                let id = commands
                    .spawn(TrackSegment {
                        p0: *q0,
                        p1: *q1,
                        p2: *q2,
                        p3: *q3,
                        start_node: TrackConnection::None,
                        end_node: TrackConnection::None,
                    })
                    .id();
                spawned_segments.push(id);
            }

            for i in 0..spawned_segments.len() {
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
                    p0: split_segments[i].0,
                    p1: split_segments[i].1,
                    p2: split_segments[i].2,
                    p3: split_segments[i].3,
                    start_node: prev_conn,
                    end_node: next_conn,
                });
            }

            if let Some(snapped_node_ent) = builder.start_node {
                for (_, mut seg) in q_segments.iter_mut() {
                    if seg.start_node == TrackConnection::LoneNode(snapped_node_ent) {
                        seg.start_node =
                            TrackConnection::Segment(spawned_segments.first().copied().unwrap());
                    } else if seg.end_node == TrackConnection::LoneNode(snapped_node_ent) {
                        seg.end_node =
                            TrackConnection::Segment(spawned_segments.first().copied().unwrap());
                    }
                }
            }

            if let Some(snapped_node_ent) = snapped_end_node {
                for (_, mut seg) in q_segments.iter_mut() {
                    if seg.start_node == TrackConnection::LoneNode(snapped_node_ent) {
                        seg.start_node =
                            TrackConnection::Segment(spawned_segments.last().copied().unwrap());
                    } else if seg.end_node == TrackConnection::LoneNode(snapped_node_ent) {
                        seg.end_node =
                            TrackConnection::Segment(spawned_segments.last().copied().unwrap());
                    }
                }
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
            let pos = eval_bezier(segment.p0, segment.p1, segment.p2, segment.p3, t);
            let dist = pos.distance(cursor_pos);

            if dist < min_dist {
                min_dist = dist;
                best_match = Some((entity, dist, t));
            }
        }
    }

    best_match
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
