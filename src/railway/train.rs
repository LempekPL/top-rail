use crate::camera::MainCamera;
use crate::railway::track::TrackSegment;
use crate::state_manager::PlayingState;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

#[derive(Default)]
pub struct TrainPlugin;

impl Plugin for TrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                spawn_train,
                drive_controls.run_if(in_state(PlayingState::Drive)),
                // move_trains,
            ),
        );
    }
}

#[derive(Component)]
pub struct Train {
    pub velocity: f32,
    pub t_pos: f32,
    pub current_track: Entity,
    pub logical_dir: f32,
}

#[allow(dead_code, unused)]
fn spawn_train(
    mut commands: Commands,
    r_mouse: Res<ButtonInput<MouseButton>>,
    s_window: Single<&Window, With<PrimaryWindow>>,
    s_camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    q_segments: Query<(Entity, &TrackSegment)>,
) {
    return;
    // if !r_mouse.just_pressed(MouseButton::Left) {
    //     return;
    // }
    //
    // let (camera, camera_transform) = *s_camera;
    // let Some(cursor_pos) = s_window
    //     .cursor_position()
    //     .and_then(|c| camera.viewport_to_world_2d(camera_transform, c).ok())
    // else {
    //     return;
    // };
    //
    // let mut closest_entity = None;
    // let mut min_dist = f32::MAX;
    // let mut closest_t = 0.0;
    //
    // let mut closest_pos = Vec2::ZERO;
    // let mut closest_tangent = Vec2::ZERO;
    //
    // for (entity, segment) in q_segments.iter() {
    //     for i in 0..=10 {
    //         let t = i as f32 / 10.0;
    //         let pos = bezier::eval(segment.p0, segment.p1, segment.p2, segment.p3, t);
    //         let dist = pos.distance(cursor_pos);
    //
    //         if dist < min_dist {
    //             min_dist = dist;
    //             closest_entity = Some(entity);
    //             closest_t = t;
    //             closest_pos = pos;
    //             closest_tangent =
    //                 bezier::derivative(segment.p0, segment.p1, segment.p2, segment.p3, t);
    //         }
    //     }
    // }
    //
    // if min_dist < 40.0 {
    //     if let Some(track_ent) = closest_entity {
    //         let mut start_rotation = Quat::IDENTITY;
    //         if closest_tangent.length_squared() > 0.0 {
    //             start_rotation = Quat::from_rotation_z(closest_tangent.y.atan2(closest_tangent.x));
    //         }
    //
    //         commands.spawn((
    //             Train {
    //                 velocity: 0.0,
    //                 t_pos: closest_t,
    //                 current_track: track_ent,
    //                 logical_dir: 1.0,
    //             },
    //             Sprite {
    //                 color: Color::srgb(1.0, 0.2, 0.2),
    //                 custom_size: Some(Vec2::new(30.0, 14.0)),
    //                 ..default()
    //             },
    //             Transform {
    //                 translation: closest_pos.extend(1.0),
    //                 rotation: start_rotation,
    //                 ..default()
    //             },
    //         ));
    //     }
    // }
}

fn drive_controls(r_keyboard: Res<ButtonInput<KeyCode>>, mut q_trains: Query<&mut Train>) {
    for mut train in q_trains.iter_mut() {
        if r_keyboard.pressed(KeyCode::ArrowUp) || r_keyboard.pressed(KeyCode::KeyE) {
            train.velocity = 150.0;
        } else if r_keyboard.pressed(KeyCode::ArrowDown) || r_keyboard.pressed(KeyCode::KeyQ) {
            train.velocity = -150.0;
        } else {
            train.velocity = 0.0;
        }
    }
}

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
