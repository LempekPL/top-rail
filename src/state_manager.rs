use crate::controls::Controls;
use bevy::prelude::*;
use std::cmp::PartialEq;

pub struct StateManagerPlugin;

impl Plugin for StateManagerPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>();
        app.add_sub_state::<PauseUiState>();
        app.add_sub_state::<MenuUiState>();
        app.add_sub_state::<PlayingState>();
        app.add_systems(OnEnter(MenuUiState::Main), despawn_when_main_menu);
        app.add_systems(Startup, setup_railway_text_state.spawn());
        app.add_systems(
            Update,
            (
                paused_change_state.run_if(in_state(GameState::Paused)),
                playing_change_state.run_if(in_state(GameState::Playing)),
                playing_text_update.run_if(state_changed::<PlayingState>),
            ),
        );
    }
}

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    #[default]
    MainMenu,
    Playing,
    Paused,
}

#[derive(SubStates, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[source(GameState = GameState::Paused)]
pub enum PauseUiState {
    #[default]
    Pause,
    Settings,
}

#[derive(SubStates, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[source(GameState = GameState::MainMenu)]
pub enum MenuUiState {
    #[default]
    Main,
    Settings,
}

#[derive(SubStates, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[source(GameState = GameState::Playing)]
pub enum PlayingState {
    #[default]
    None,
    Build,
    Bulldoze,
    Spawn,
    Drive,
}

#[derive(Component, Default, Clone)]
pub struct DespawnWhenMainMenu;

fn despawn_when_main_menu(
    mut commands: Commands,
    q_to_despawn: Query<Entity, With<DespawnWhenMainMenu>>,
) {
    for entity in q_to_despawn.iter() {
        commands.entity(entity).despawn();
    }
}

#[derive(Component, Default, Clone)]
struct RailwayStateText;

fn setup_railway_text_state() -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(0.0),
        }
        Text("") RailwayStateText
    }
}

fn playing_text_update(
    s_railway: Res<State<PlayingState>>,
    mut s_text: Single<&mut Text, With<RailwayStateText>>,
) {
    match s_railway.get() {
        PlayingState::Build => s_text.0 = "Build".to_string(),
        PlayingState::Bulldoze => s_text.0 = "Bulldoze".to_string(),
        PlayingState::Spawn => s_text.0 = "Spawn".to_string(),
        PlayingState::Drive => s_text.0 = "Drive".to_string(),
        PlayingState::None => s_text.0 = "".to_string(),
    }
}

fn paused_change_state(mut game_next: ResMut<NextState<GameState>>, controls: Controls) {
    if controls.just_pressed(|k| k.esc) {
        game_next.set(GameState::Playing);
    }
}

fn playing_change_state(
    play_current: Res<State<PlayingState>>,
    mut play_next: ResMut<NextState<PlayingState>>,
    mut game_next: ResMut<NextState<GameState>>,
    controls: Controls,
) {
    if controls.just_pressed(|k| k.esc) {
        if play_current.get() == &PlayingState::None {
            game_next.set(GameState::Paused);
        } else {
            play_next.set(PlayingState::None);
        }
    } else if controls.just_pressed(|k| k.build) {
        if play_current.get() == &PlayingState::Build {
            play_next.set(PlayingState::None);
        } else {
            play_next.set(PlayingState::Build);
        }
    } else if controls.just_pressed(|k| k.bulldoze) {
        if play_current.get() == &PlayingState::Bulldoze {
            play_next.set(PlayingState::None);
        } else {
            play_next.set(PlayingState::Bulldoze);
        }
    }
}
