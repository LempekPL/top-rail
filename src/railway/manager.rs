use crate::controls::Controls;
use bevy::prelude::*;
use std::cmp::PartialEq;

#[derive(Default)]
pub struct RailwayManagerPlugin;

impl Plugin for RailwayManagerPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<RailwayState>();
        app.add_systems(Startup, setup_text_state.spawn());
        app.add_systems(
            Update,
            (
                change_state,
                update_state.run_if(state_changed::<RailwayState>),
            ),
        );
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
pub enum RailwayState {
    #[default]
    None,
    Build,
    Bulldoze,
    Spawn,
    Drive,
}

#[derive(Component, Default, Clone)]
struct RailwayStateText;

fn setup_text_state() -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(0.0),
        }
        Text("Build") RailwayStateText
    }
}

fn update_state(
    s_railway: Res<State<RailwayState>>,
    mut s_text: Single<&mut Text, With<RailwayStateText>>,
) {
    match s_railway.get() {
        RailwayState::Build => s_text.0 = "Build".to_string(),
        RailwayState::Bulldoze => s_text.0 = "Bulldoze".to_string(),
        RailwayState::Spawn => s_text.0 = "Spawn".to_string(),
        RailwayState::Drive => s_text.0 = "Drive".to_string(),
        RailwayState::None => s_text.0 = "".to_string(),
    }
}

fn change_state(
    r_current: Res<State<RailwayState>>,
    mut r_next: ResMut<NextState<RailwayState>>,
    controls: Controls,
) {
    if controls.just_pressed(|k| k.esc) {
        r_next.set(RailwayState::None);
    } else if controls.just_pressed(|k| k.build) {
        if r_current.get() == &RailwayState::Build {
            r_next.set(RailwayState::None);
        } else {
            r_next.set(RailwayState::Build);
        }
    } else if controls.just_pressed(|k| k.bulldoze) {
        if r_current.get() == &RailwayState::Bulldoze {
            r_next.set(RailwayState::None);
        } else {
            r_next.set(RailwayState::Bulldoze);
        }
    }
}
