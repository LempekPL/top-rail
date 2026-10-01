use bevy::app::{App, Plugin, Update};

pub mod slider;

pub struct ComponentsPlugin;

impl Plugin for ComponentsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (slider::update_slider_data, slider::update_slider_style));
    }
}