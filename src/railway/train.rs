use crate::camera::WindowCamera;
use crate::railway::track::Track;
use crate::state_manager::PlayingState;
use bevy::prelude::*;

#[derive(Default)]
pub struct TrainPlugin;

impl Plugin for TrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                spawn_train.run_if(in_state(PlayingState::Spawn)),
                // drive_controls.run_if(in_state(PlayingState::Drive)),
                // move_trains,
            ),
        );
    }
}

#[derive(Component)]
pub struct SelectedTrain;

#[derive(Component, Debug, Clone)]
pub struct Train {
    pub current_segment: Entity,
    pub t: f32,
    pub speed: f32,
    pub direction: i8,
}

// in the future spawning will be only available in depot
fn spawn_train(
    mut commands: Commands,
    r_mouse: Res<ButtonInput<MouseButton>>,
    camera: WindowCamera,
    track: Track,
) {
    if !r_mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(cursor_pos) = camera.get_world_cursor() else {
        return;
    };

    let mut closest_segment = None;
    let mut closest_dist = 20.0;
    let mut best_t = 0.0;
    let mut spawn_pos = Vec2::ZERO;
    let mut spawn_tangent = Vec2::ZERO;

    for (seg_ent, curve) in track.iter_curves() {
        let samples = 20;
        for i in 0..=samples {
            let t = i as f32 / samples as f32;
            let point = curve.position(t);
            let dist = point.distance(cursor_pos);

            if dist < closest_dist {
                closest_dist = dist;
                closest_segment = Some(seg_ent);
                best_t = t;
                spawn_pos = point;
                spawn_tangent = curve.velocity(t).normalize_or_zero();
            }
        }
    }

    let Some(segment_ent) = closest_segment else {
        return;
    };

    commands.spawn((
        Sprite {
            color: Color::srgb(1.0, 0.4, 0.0),
            custom_size: Some(Vec2::new(50.0, 18.0)),
            ..default()
        },
        Transform::from_translation(spawn_pos.extend(5.0))
            .with_rotation(Quat::from_rotation_z(spawn_tangent.to_angle())),
        Train {
            current_segment: segment_ent,
            t: best_t,
            speed: 0.1,
            direction: 1,
        },
    ));
}



// fn drive_controls(r_keyboard: Res<ButtonInput<KeyCode>>, mut q_trains: Query<&mut Train>) {
//     for mut train in q_trains.iter_mut() {
//         if r_keyboard.pressed(KeyCode::ArrowUp) || r_keyboard.pressed(KeyCode::KeyE) {
//             train.velocity = 150.0;
//         } else if r_keyboard.pressed(KeyCode::ArrowDown) || r_keyboard.pressed(KeyCode::KeyQ) {
//             train.velocity = -150.0;
//         } else {
//             train.velocity = 0.0;
//         }
//     }
// }

// fn move_trains(
//     time: Res<Time>,
//     mut q_trains: Query<(&mut Train, &mut Transform)>,
//     q_segments: Query<&TrackSegment>,
// ) {
//     for (mut train, mut transform) in q_trains.iter_mut() {
//         if train.velocity == 0.0 {
//             continue;
//         }
//         let Ok(segment) = q_segments.get(train.current_track) else {
//             continue;
//         };
//         let approx_length = segment.p0.distance(segment.p1)
//             + segment.p1.distance(segment.p2)
//             + segment.p2.distance(segment.p3);
//         let delta_t = (train.velocity * time.delta_secs()) / approx_length.max(1.0);
//         train.t_pos += delta_t * train.logical_dir;
//
//         if train.t_pos > 1.0 {
//             match &segment.end_node {
//                 TrackConnection::Segment(next_ent) => {
//                     if let Ok(next_seg) = q_segments.get(*next_ent) {
//                         let exit_tangent =
//                             bezier::derivative(segment.p0, segment.p1, segment.p2, segment.p3, 1.0)
//                                 .normalize_or_zero();
//                         let nose_dir = exit_tangent * train.logical_dir;
//                         let remainder = train.t_pos - 1.0;
//                         train.current_track = *next_ent;
//                         if segment.p3.distance_squared(next_seg.p0)
//                             < segment.p3.distance_squared(next_seg.p3)
//                         {
//                             train.t_pos = remainder;
//                         } else {
//                             train.t_pos = 1.0 - remainder;
//                         }
//                         let entry_tangent = bezier::derivative(
//                             next_seg.p0,
//                             next_seg.p1,
//                             next_seg.p2,
//                             next_seg.p3,
//                             train.t_pos,
//                         )
//                         .normalize_or_zero();
//                         train.logical_dir = if nose_dir.dot(entry_tangent) >= 0.0 {
//                             1.0
//                         } else {
//                             -1.0
//                         };
//                     }
//                 }
//                 _ => {
//                     train.velocity = 0.0;
//                     train.t_pos = 1.0;
//                 }
//             }
//         } else if train.t_pos < 0.0 {
//             match &segment.start_node {
//                 TrackConnection::Segment(next_ent) => {
//                     if let Ok(next_seg) = q_segments.get(*next_ent) {
//                         let exit_tangent =
//                             bezier::derivative(segment.p0, segment.p1, segment.p2, segment.p3, 0.0)
//                                 .normalize_or_zero();
//                         let nose_dir = exit_tangent * train.logical_dir;
//                         let remainder = train.t_pos.abs();
//                         train.current_track = *next_ent;
//                         if segment.p0.distance_squared(next_seg.p3)
//                             < segment.p0.distance_squared(next_seg.p0)
//                         {
//                             train.t_pos = 1.0 - remainder;
//                         } else {
//                             train.t_pos = remainder;
//                         }
//                         let entry_tangent = bezier::derivative(
//                             next_seg.p0,
//                             next_seg.p1,
//                             next_seg.p2,
//                             next_seg.p3,
//                             train.t_pos,
//                         )
//                         .normalize_or_zero();
//                         train.logical_dir = if nose_dir.dot(entry_tangent) >= 0.0 {
//                             1.0
//                         } else {
//                             -1.0
//                         };
//                     }
//                 }
//                 _ => {
//                     train.velocity = 0.0;
//                     train.t_pos = 0.0;
//                 }
//             }
//         }
//
//         if let Ok(active_segment) = q_segments.get(train.current_track) {
//             let pos = bezier::eval(
//                 active_segment.p0,
//                 active_segment.p1,
//                 active_segment.p2,
//                 active_segment.p3,
//                 train.t_pos.clamp(0.0, 1.0),
//             );
//             transform.translation = pos.extend(1.0);
//
//             let tangent = bezier::derivative(
//                 active_segment.p0,
//                 active_segment.p1,
//                 active_segment.p2,
//                 active_segment.p3,
//                 train.t_pos.clamp(0.0, 1.0),
//             );
//
//             if tangent.length_squared() > 0.0 {
//                 let visual_dir = tangent * train.logical_dir;
//                 transform.rotation = Quat::from_rotation_z(visual_dir.y.atan2(visual_dir.x));
//             }
//         }
//     }
// }
