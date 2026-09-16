use bevy::app::plugin_group;

pub(crate) mod track;
mod train;

plugin_group! {
    pub struct RailwayPlugin {
        track:::TrackPlugin,
        train:::TrainPlugin,
    }
}