use bevy::prelude::*;

#[derive(Default)]
pub struct RailwayManagerPlugin;

impl Plugin for RailwayManagerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RailwaySettings>();
        app.add_systems(Startup, mode_text.spawn());
        app.add_systems(Update, change_mode);
    }
}

#[derive(Default)]
pub enum RailwayMode {
    #[default]
    Build,
    Bulldoze,
    Spawn,
    Drive,
}

#[derive(Resource, Default)]
pub struct RailwaySettings {
    mode: RailwayMode,
}

#[derive(Component, Default, Clone)]
struct ModeText;

fn mode_text() -> impl Scene {
    bsn! {
        Text("Build") ModeText
    }
}

fn change_mode(
    mut r_options: ResMut<RailwaySettings>,
    r_keyboard: Res<ButtonInput<KeyCode>>,
    mut s_text: Single<&mut Text, With<ModeText>>,
) {
    if r_keyboard.just_pressed(KeyCode::KeyE) {
        r_options.mode = RailwayMode::Drive;
        s_text.0 = "Drive".to_string();
    }
    if r_keyboard.just_pressed(KeyCode::KeyC) {
        r_options.mode = RailwayMode::Spawn;
        s_text.0 = "Spawn".to_string();
    }
    if r_keyboard.just_pressed(KeyCode::KeyV) {
        r_options.mode = RailwayMode::Build;
        s_text.0 = "Build".to_string();
    }
    if r_keyboard.just_pressed(KeyCode::KeyB) {
        r_options.mode = RailwayMode::Bulldoze;
        s_text.0 = "Bulldoze".to_string();
    }
}
