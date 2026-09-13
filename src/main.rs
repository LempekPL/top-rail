#![allow(dead_code)]

mod camera;
mod railway;
mod util;

use crate::camera::CameraPlugin;
use crate::railway::RailwayPlugin;
// use bevy::input::common_conditions::input_toggle_active;
use bevy::prelude::*;
// use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};
use bevy::dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin, FrameTimeGraphConfig};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(FpsOverlayPlugin { config: FpsOverlayConfig {
            text_config: TextFont {
                font_size: FontSize::Px(10.0),
                ..default()
            },
            text_color: Color::srgb(0.0, 1.0, 0.0),
            enabled: true,
            refresh_interval: core::time::Duration::from_millis(100),
            frame_time_graph_config: FrameTimeGraphConfig {
                enabled: false,
                min_fps: 30.0,
                target_fps: 144.0,
            },
        } })
        .add_plugins(CameraPlugin)
        .add_plugins(RailwayPlugin)
        // .add_plugins(EguiPlugin::default())
        // .add_plugins(
        //     WorldInspectorPlugin::default().run_if(input_toggle_active(true, KeyCode::Escape)),
        // )
        .run();
}
