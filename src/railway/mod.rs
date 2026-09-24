use bevy::app::plugin_group;

mod graphics;
pub mod track;
mod train;

plugin_group! {
    pub struct RailwayPlugin {
        track:::TrackPlugin,
        train:::TrainPlugin,
        graphics:::GraphicsPlugin,
    }
}
