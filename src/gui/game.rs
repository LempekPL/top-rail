use crate::state_manager::{GameState, PlayingState};
use bevy::feathers::controls::FeathersButton;
use bevy::feathers::rounded_corners::RoundedCorners;
use bevy::prelude::*;
use bevy::ui_widgets::Activate;

pub struct GameUiPlugin;

impl Plugin for GameUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_build_menu);
        app.add_systems(OnEnter(GameState::Playing), show_build_menu);
        app.add_systems(OnExit(GameState::Playing), hide_build_menu);
    }
}

#[derive(Component, Clone, Default)]
struct BuildBoxUi;

fn setup_build_menu(mut commands: Commands) {
    commands.spawn_scene(bsn! {
        Node {
            margin: UiRect::horizontal(auto()),
            bottom: px(10),
            display: Display::None,
            column_gap: px(8),
            flex_direction: FlexDirection::Row,
            position_type: PositionType::Absolute,
        }
        BuildBoxUi
        template_value(Interaction::None)
        Children [
            @FeathersButton {
                @caption: bsn! {
                    Node {
                        height: px(20),
                    }
                    ImageNode {
                        image: "icons/track.png"
                    }
                }
            }
            Node {
                height: px(32),
                width: px(32),
                border_radius: {RoundedCorners::All.to_border_radius(100.)},
            }
            on(|_: On<Activate>, r_current: Res<State<PlayingState>>, mut r_next: ResMut<NextState<PlayingState>>| {
                if r_current.get() == &PlayingState::Build {
                    r_next.set(PlayingState::None);
                } else {
                    r_next.set(PlayingState::Build);
                }
            }),

            @FeathersButton {
                @caption: bsn! {
                    Node {
                        height: px(20),
                    }
                    ImageNode {
                        image: "icons/bulldoze.png"
                    }
                }
            }
            Node {
                height: px(32),
                width: px(32),
                border_radius: {RoundedCorners::All.to_border_radius(100.)},
            }
            on(|_: On<Activate>, r_current: Res<State<PlayingState>>, mut r_next: ResMut<NextState<PlayingState>>| {
                if r_current.get() == &PlayingState::Bulldoze {
                    r_next.set(PlayingState::None);
                } else {
                    r_next.set(PlayingState::Bulldoze);
                }
            }),
        ]
    });
}

fn show_build_menu(mut q_build_box: Single<&mut Node, With<BuildBoxUi>>) {
    q_build_box.display = Display::Flex
}

fn hide_build_menu(mut q_build_box: Single<&mut Node, With<BuildBoxUi>>) {
    q_build_box.display = Display::None
}
