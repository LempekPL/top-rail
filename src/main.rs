#![allow(dead_code)]

mod camera;
mod railway;
mod util;
mod controls;

use crate::camera::CameraPlugin;
use crate::railway::RailwayPlugin;
use bevy::dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin, FrameTimeGraphConfig};
use bevy::prelude::*;
use bevy::window::PresentMode;
use bevy_framepace::{FramepacePlugin, FramepaceSettings, Limiter};
use crate::controls::ControlsPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: PresentMode::Mailbox,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(FpsOverlayPlugin {
            config: FpsOverlayConfig {
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
            },
        })
        .add_plugins(CameraPlugin)
        .add_plugins(RailwayPlugin)
        .add_plugins(FramepacePlugin)
        .add_plugins(ControlsPlugin)
        .add_systems(Startup, setup_framerate)
        .run();
}

fn setup_framerate(mut settings: ResMut<FramepaceSettings>) {
    settings.limiter = Limiter::from_framerate(165.0);
}
