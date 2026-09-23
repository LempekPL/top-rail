use bevy::app::plugin_group;

pub mod track;
mod train;
mod graphics;

plugin_group! {
    pub struct RailwayPlugin {
        track:::TrackPlugin,
        train:::TrainPlugin,
        graphics:::GraphicsPlugin,
    }
}