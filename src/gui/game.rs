use crate::camera::WindowCamera;
use crate::gui::components::slider::LeverSlider;
use crate::railway::train::{DrivingTrain, SelectedTrain, Train};
use crate::state_manager::{GameState, PlayingState};
use bevy::feathers::controls::FeathersButton;
use bevy::feathers::rounded_corners::RoundedCorners;
use bevy::prelude::*;
use bevy::ui_widgets::Activate;

pub struct GameUiPlugin;

impl Plugin for GameUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_build_menu, spawn_train_context_menu));
        app.add_systems(OnEnter(GameState::Playing), display_flex_menu::<BuildBoxUi>);
        app.add_systems(
            OnExit(GameState::Playing),
            (
                display_none_menu::<BuildBoxUi>,
                display_none_menu::<TrainContextMenu>,
                display_none_menu::<TrainControlsMenu>,
            ),
        );
        app.add_systems(Update, move_train_context_menu);
    }
}

#[derive(Component, Clone, Default)]
struct BuildBoxUi;

#[derive(Component, Clone, Default)]
struct TrainContextMenu;

#[derive(Component, Clone, Default)]
struct TrainControlsMenu;

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

            @FeathersButton {
                @caption: bsn! {
                    Node {
                        height: px(20),
                    }
                    ImageNode {
                        image: "icons/train.png"
                    }
                }
            }
            Node {
                height: px(32),
                width: px(32),
                border_radius: {RoundedCorners::All.to_border_radius(100.)},
            }
            on(|_: On<Activate>, r_current: Res<State<PlayingState>>, mut r_next: ResMut<NextState<PlayingState>>| {
                if r_current.get() == &PlayingState::Spawn {
                    r_next.set(PlayingState::None);
                } else {
                    r_next.set(PlayingState::Spawn);
                }
            }),
        ]
    });
}

fn spawn_train_context_menu(mut commands: Commands) {
    commands.spawn_scene(bsn! {
        Node {
            position_type: PositionType::Absolute,
            top: px(10)
            left: px(10),
            display: Display::None,
            row_gap: px(8),
            flex_direction: FlexDirection::Column,
            padding: px(4),
            border_radius: {RoundedCorners::All.to_border_radius(8.)},
        }
        BackgroundColor(Color::srgba(0.35, 0.35, 0.35, 1.0))
        TrainContextMenu
        template_value(Interaction::None)
        Children [
            @FeathersButton {
                @caption: bsn! {
                    Text("Drive")
                }
            }
            Node {
                padding: px(12),
                border_radius: {RoundedCorners::All.to_border_radius(100.)},
            }
            on(|_: On<Activate>, mut commands: Commands, train: Single<Entity, With<SelectedTrain>>| {
                commands.entity(*train).remove::<SelectedTrain>().insert(DrivingTrain);

            }),

            @FeathersButton {
                @caption: bsn! {
                    Text("Despawn")
                }
            }
            Node {
                padding: px(12),
                border_radius: {RoundedCorners::All.to_border_radius(100.)},
            }
            on(|_: On<Activate>, mut commands: Commands, train: Single<Entity, With<SelectedTrain>>| {
                commands.entity(*train).despawn();
            }),
        ]
    });
}

fn move_train_context_menu(
    q_trains: Query<&GlobalTransform, With<SelectedTrain>>,
    camera: WindowCamera,
    mut q_menu: Query<(&mut Node, Option<&ComputedNode>), With<TrainContextMenu>>,
) {
    let Ok(train_global_transform) = q_trains.single() else {
        for (mut node, _) in q_menu.iter_mut() {
            node.display = Display::None;
        }
        return;
    };
    let Some(screen_pos) = camera.world_to_viewport(*train_global_transform) else {
        return;
    };

    for (mut node, computed_node) in q_menu.iter_mut() {
        node.display = Display::Flex;

        let (menu_width, menu_height) = if let Some(computed) = computed_node {
            (computed.size().x, computed.size().y)
        } else {
            (150.0, 100.0)
        };

        let max_x = (camera.window.width() - menu_width).max(0.0);
        let max_y = (camera.window.height() - menu_height).max(0.0);

        let final_x = (screen_pos.x + 20.0).clamp(0.0, max_x);
        let final_y = (screen_pos.y - 20.0).clamp(0.0, max_y);

        node.left = Val::Px(final_x);
        node.top = Val::Px(final_y);
    }
}

fn spawn_controls_when_drive_train(
    mut commands: Commands,
    train: Single<Has<DrivingTrain>, With<DrivingTrain>>,
) {
}

#[derive(Component, Default, Clone)]
pub struct DriveControl;
#[derive(Component, Default, Clone)]
pub struct BreakControl;

fn spawn_train_controls_menu(mut commands: Commands, _: Train) {
    commands.spawn_scene(bsn! {
        Node {
            width: px(150),
            height: px(300),
            position_type: PositionType::Absolute,
            bottom: px(10)
            right: px(10),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            display: Display::None,
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
            border_radius: {RoundedCorners::All.to_border_radius(8.)},
        }
        template_value(Interaction::None)
        TrainControlsMenu
        BackgroundColor(Color::srgb(0.5, 0.5, 0.5))
        Children [
            Text("Train 1"),
            BackgroundColor(Color::srgb(0.5, 0.5, 0.5))
            Node {
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceAround,
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
            }
            Children [
                @LeverSlider {
                    @min: -100.,
                    @max: 100.,
                    @value: 0.,
                    @thumb_color: Color::srgb(0.65, 0.65, 0.65),
                }
                DriveControl,

                // @LeverSlider {
                //     @min: -100.,
                //     @max: 100.,
                //     @value: 0.,
                //     @thumb_color: Color::srgb(0.75, 0.2, 0.2),
                // }
                // BreakControl,
            ]
        ]
    });
}

fn display_flex_menu<MenuComponent: Component>(mut q_box: Query<&mut Node, With<MenuComponent>>) {
    for mut node in q_box.iter_mut() {
        node.display = Display::Flex;
    }
}

fn display_none_menu<MenuComponent: Component>(mut q_box: Query<&mut Node, With<MenuComponent>>) {
    for mut node in q_box.iter_mut() {
        node.display = Display::None;
    }
}
