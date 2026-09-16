use crate::state_manager::PlayingState;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;

pub struct CameraPlugin;

const CAMERA_SPEED: f32 = 4.0;
const CAMERA_SPEED_UP: f32 = 10.0;
const CAMERA_ZOOM_SPEED: f32 = 0.25;
const CAMERA_ZOOM_MIN: f32 = CAMERA_ZOOM_SPEED;
const CAMERA_ZOOM_MAX: f32 = CAMERA_ZOOM_SPEED * (4. + 10.);

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera);
        app.add_systems(Update, move_camera);
    }
}

#[derive(Component)]
pub struct MainCamera;

fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, MainCamera));
}

fn move_camera(
    mut q_camera: Query<(&mut Transform, &mut Projection), With<MainCamera>>,
    r_mouse: Res<ButtonInput<MouseButton>>,
    r_state: Option<Res<State<PlayingState>>>,
    mut m_mouse_scroll: MessageReader<MouseWheel>,
    mut m_mouse_move: MessageReader<MouseMotion>,
) {
    let Ok((mut transform, mut projection)) = q_camera.single_mut() else {
        return;
    };
    let Projection::Orthographic(ref mut ortho) = *projection else {
        return;
    };
    for scroll in m_mouse_scroll.read() {
        if scroll.y < 0. {
            ortho.scale += CAMERA_ZOOM_SPEED;
        } else if scroll.y > 0. {
            ortho.scale -= CAMERA_ZOOM_SPEED;
        }
        ortho.scale = ortho.scale.min(CAMERA_ZOOM_MAX).max(CAMERA_ZOOM_MIN);
    }

    let left_move = r_state.map_or(false, |state| matches!(state.get(), PlayingState::None | PlayingState::Drive));
    if (left_move && r_mouse.pressed(MouseButton::Left)) || r_mouse.pressed(MouseButton::Middle) {
        for mouse_move in m_mouse_move.read() {
            transform.translation +=
                (mouse_move.delta * Vec2::new(-1., 1.) * ortho.scale).extend(0.0);
        }
    }
}
