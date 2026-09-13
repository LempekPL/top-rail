#![allow(dead_code)]

mod camera;
mod railway;
mod util;

use crate::camera::CameraPlugin;
use crate::railway::RailwayPlugin;
// use bevy::input::common_conditions::input_toggle_active;
use bevy::prelude::*;
// use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(CameraPlugin)
        .add_plugins(RailwayPlugin)
        // .add_plugins(EguiPlugin::default())
        // .add_plugins(
        //     WorldInspectorPlugin::default().run_if(input_toggle_active(true, KeyCode::Escape)),
        // )
        .run();
}
