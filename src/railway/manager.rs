use bevy::prelude::*;
use std::cmp::PartialEq;

#[derive(Default)]
pub struct RailwayManagerPlugin;

impl Plugin for RailwayManagerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RailwaySettings>();
        app.add_systems(Startup, mode_text.spawn());
        app.add_systems(Update, (change_mode, update_mode.run_if(resource_changed::<RailwaySettings>)));
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub enum RailwayMode {
    #[default]
    Build,
    Bulldoze,
    Spawn,
    Drive,
}

#[derive(Resource)]
pub struct RailwaySettings {
    pub mode: RailwayMode,
    pub prev_mode: RailwayMode,
}

impl Default for RailwaySettings {
    fn default() -> Self {
        Self {
            mode: RailwayMode::Build,
            prev_mode: RailwayMode::Bulldoze,
        }
    }
}

#[derive(Component, Default, Clone)]
struct ModeText;

fn mode_text() -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(0.0),
        }
        Text("Build") ModeText
    }
}

fn update_mode(
    r_options: Res<RailwaySettings>,
    mut s_text: Single<&mut Text, With<ModeText>>,
) {
    match r_options.mode {
        RailwayMode::Build => s_text.0 = "Build".to_string(),
        RailwayMode::Bulldoze => s_text.0 = "Bulldoze".to_string(),
        RailwayMode::Spawn => s_text.0 = "Spawn".to_string(),
        RailwayMode::Drive => s_text.0 = "Drive".to_string(),
    }
}

fn change_mode(
    mut r_options: ResMut<RailwaySettings>,
    r_keyboard: Res<ButtonInput<KeyCode>>,
) {
    let tmp = r_options.mode.clone();
    if r_keyboard.just_pressed(KeyCode::Digit1) {
        if r_options.mode == RailwayMode::Build {
            r_options.mode = r_options.prev_mode.clone();
        } else {
            r_options.mode = RailwayMode::Build;
        }
    }
    if r_keyboard.just_pressed(KeyCode::KeyB) {
        if r_options.mode == RailwayMode::Bulldoze {
            if r_options.prev_mode == RailwayMode::Spawn {
                r_options.mode = RailwayMode::Drive;
            } else {
                r_options.mode = r_options.prev_mode.clone();
            }
        } else {
            r_options.mode = RailwayMode::Bulldoze;
        }
    }
    if r_keyboard.just_pressed(KeyCode::Digit2) {
        r_options.mode = RailwayMode::Drive;
    }
    if r_keyboard.just_pressed(KeyCode::Digit3) {
        r_options.mode = RailwayMode::Spawn;
    }
    if tmp != r_options.mode {
        r_options.prev_mode = tmp;
    }
}
