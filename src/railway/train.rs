use bevy::prelude::*;

#[derive(Default)]
pub struct TrainPlugin;

impl Plugin for TrainPlugin {
    fn build(&self, app: &mut App) {

    }
}

#[derive(Component)]
pub struct Train {

}