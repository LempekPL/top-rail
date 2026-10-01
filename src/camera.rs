use crate::consts::camera::{ZOOM_MAX, ZOOM_MIN, ZOOM_SPEED};
use crate::state_manager::PlayingState;
use bevy::ecs::system::SystemParam;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera);
        app.add_systems(Update, move_camera);
    }
}

#[derive(SystemParam)]
pub struct WindowCamera<'w, 's> {
    pub window: Single<'w, 's, &'static Window, With<PrimaryWindow>>,
    pub camera: Single<'w, 's, (&'static Camera, &'static GlobalTransform), With<MainCamera>>,
}

impl<'w, 's> WindowCamera<'w, 's> {
    pub fn get_world_cursor(&self) -> Option<Vec2> {
        let (camera, camera_transform) = *self.camera;
        let Some(cursor_world_pos) = self
            .window
            .cursor_position()
            .and_then(|cursor| camera.viewport_to_world_2d(camera_transform, cursor).ok())
        else {
            return None;
        };
        Some(cursor_world_pos)
    }

    pub fn world_to_viewport(&self, global_transform: GlobalTransform) -> Option<Vec2> {
        let (camera, camera_transform) = *self.camera;
        let Ok(screen_pos) =
            camera.world_to_viewport(camera_transform, global_transform.translation())
        else {
            return None;
        };
        Some(screen_pos)
    }
}

#[derive(Component)]
pub struct MainCamera;

fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, MainCamera, Transform::from_xyz(0.0, 0.0, 500.0)));
}

fn move_camera(
    mut q_camera: Query<(&mut Transform, &mut Projection), With<MainCamera>>,
    r_mouse: Res<ButtonInput<MouseButton>>,
    r_state: Option<Res<State<PlayingState>>>,
    mut m_mouse_scroll: MessageReader<MouseWheel>,
    mut m_mouse_move: MessageReader<MouseMotion>,
    q_interaction: Query<&Interaction, With<Node>>,
) {
    if r_state.is_none() {
        return;
    }
    let Ok((mut transform, mut projection)) = q_camera.single_mut() else {
        return;
    };
    let Projection::Orthographic(ref mut ortho) = *projection else {
        return;
    };

    let is_pointer_over_ui = q_interaction
        .iter()
        .any(|interaction| *interaction != Interaction::None);

    if !is_pointer_over_ui {
        for scroll in m_mouse_scroll.read() {
            if scroll.y < 0. {
                ortho.scale += ZOOM_SPEED;
            } else if scroll.y > 0. {
                ortho.scale -= ZOOM_SPEED;
            }
            ortho.scale = ortho.scale.min(ZOOM_MAX).max(ZOOM_MIN);
        }
    } else {
        let _ = m_mouse_scroll.read().count();
    }

    let left_move = r_state.map_or(false, |state| {
        matches!(state.get(), PlayingState::None | PlayingState::Drive)
    });
    if !is_pointer_over_ui
        && ((left_move && r_mouse.pressed(MouseButton::Left))
            || r_mouse.pressed(MouseButton::Middle))
    {
        for mouse_move in m_mouse_move.read() {
            transform.translation +=
                (mouse_move.delta * Vec2::new(-1., 1.) * ortho.scale).extend(0.0);
        }
    } else {
        let _ = m_mouse_move.read().count();
    }
}
