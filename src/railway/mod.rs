use bevy::app::plugin_group;

mod track;
mod train;
pub mod manager;

plugin_group! {
    pub struct RailwayPlugin {
        track:::TrackPlugin,
        train:::TrainPlugin,
        manager:::RailwayManagerPlugin,
    }
}