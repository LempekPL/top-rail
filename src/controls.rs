use bevy::app::{App, Plugin};
use bevy::ecs::system::SystemParam;
use bevy::input::ButtonInput;
use bevy::prelude::*;

pub struct ControlsPlugin;

impl Plugin for ControlsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Keymap>();
    }
}

#[derive(Resource, Debug, Clone)]
pub struct Keymap {
    pub esc: KeyCode,
    pub up: KeyCode,
    pub down: KeyCode,
    pub left: KeyCode,
    pub right: KeyCode,
    pub build: KeyCode,
    pub bulldoze: KeyCode,
    pub not_snap: KeyCode,
    pub debug: KeyCode,
}

impl Default for Keymap {
    fn default() -> Self {
        Self {
            esc: KeyCode::Escape,
            up: KeyCode::KeyW,
            down: KeyCode::KeyS,
            left: KeyCode::KeyA,
            right: KeyCode::KeyD,
            build: KeyCode::Digit1,
            bulldoze: KeyCode::KeyB,
            not_snap: KeyCode::KeyC,
            debug: KeyCode::F3,
        }
    }
}

#[derive(SystemParam)]
pub struct Controls<'w> {
    keyboard: Res<'w, ButtonInput<KeyCode>>,
    keymap: Res<'w, Keymap>,
}

impl<'w> Controls<'w> {
    pub fn pressed(&self, get_key: fn(&Keymap) -> KeyCode) -> bool {
        self.keyboard.pressed(get_key(&self.keymap))
    }

    pub fn just_pressed(&self, get_key: fn(&Keymap) -> KeyCode) -> bool {
        self.keyboard.just_pressed(get_key(&self.keymap))
    }

    pub fn just_released(&self, get_key: fn(&Keymap) -> KeyCode) -> bool {
        self.keyboard.just_released(get_key(&self.keymap))
    }
}