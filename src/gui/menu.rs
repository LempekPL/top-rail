use crate::camera::MainCamera;
use crate::save_load::{LoadGame, SaveGame};
use crate::state_manager::{GameState, MenuUiState, PauseUiState};
use bevy::feathers::containers::{subpane_body, subpane_header};
use bevy::feathers::controls::FeathersButton;
use bevy::feathers::theme::ThemedText;
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::prelude::*;
use bevy::ui_widgets::Activate;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_main_box, show_main_menu).chain());
        app.add_systems(OnEnter(MenuUiState::Main), show_main_menu);
        app.add_systems(OnEnter(MenuUiState::Settings), show_settings_menu);
        app.add_systems(OnEnter(PauseUiState::Pause), show_pause_menu);
        app.add_systems(OnEnter(PauseUiState::Settings), show_pause_settings_menu);
        app.add_systems(OnEnter(GameState::Playing), hide_main_box);
    }
}

#[derive(Component, Clone, Default)]
struct MainBox;

fn setup_main_box(mut commands: Commands) {
    commands.spawn_scene(bsn! {
        Node {
            display: Display::None,
            width: Val::Vw(100.0),
            height: Val::Vh(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
        }
        // BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5))
        MainBox
        TabGroup
    });
}

fn show_main_box(
    commands: &mut Commands,
    q_main_box: Single<(Entity, &mut Node), With<MainBox>>,
    menu_box: impl Scene,
) {
    let (main_box_ent, ref mut main_box_node) = q_main_box.into_inner();
    main_box_node.display = Display::Flex;
    commands.entity(main_box_ent).despawn_children();
    let menu = commands.spawn_scene(menu_box).id();
    commands.entity(main_box_ent).add_child(menu);
}

fn show_main_menu(mut commands: Commands, q_main_box: Single<(Entity, &mut Node), With<MainBox>>) {
    show_main_box(&mut commands, q_main_box, main_menu());
}

fn show_settings_menu(
    mut commands: Commands,
    q_main_box: Single<(Entity, &mut Node), With<MainBox>>,
) {
    show_main_box(
        &mut commands,
        q_main_box,
        settings_menu(on(
            |_: On<Activate>, mut r_next: ResMut<NextState<MenuUiState>>| {
                r_next.set(MenuUiState::Main);
            },
        )),
    );
}

fn show_pause_menu(mut commands: Commands, q_main_box: Single<(Entity, &mut Node), With<MainBox>>) {
    show_main_box(&mut commands, q_main_box, pause_menu());
}

fn show_pause_settings_menu(
    mut commands: Commands,
    q_main_box: Single<(Entity, &mut Node), With<MainBox>>,
) {
    show_main_box(
        &mut commands,
        q_main_box,
        settings_menu(on(
            |_: On<Activate>, mut r_next: ResMut<NextState<PauseUiState>>| {
                r_next.set(PauseUiState::Pause);
            },
        )),
    );
}

fn hide_main_box(q_main_box: Single<&mut Node, With<MainBox>>) {
    let mut main_box = q_main_box.into_inner();
    main_box.display = Display::None;
}

fn main_menu() -> impl Scene {
    bsn! {
        Node {
            flex_direction: FlexDirection::Column,
            width: Val::Px(200.0),
        }
        Children [
            subpane_header() Children [
                (Text("Main Menu") ThemedText),
            ],
            subpane_body()
            Node {
                row_gap: px(8),
            }
            Children [
                @FeathersButton {
                    @caption: bsn! { Text("Start") ThemedText }
                }
                on(|_: On<Activate>, mut r_next: ResMut<NextState<GameState>>, camera: Single<&mut Transform, With<MainCamera>>| {
                    r_next.set(GameState::Playing);
                    let mut camera_pos = camera.into_inner();
                    camera_pos.translation = Vec3::new(0.,0.,0.);
                }),

                @FeathersButton {
                    @caption: bsn! { Text("Load") ThemedText }
                }
                on(|_: On<Activate>, mut r_next: ResMut<NextState<GameState>>, mut load_message: MessageWriter<LoadGame>| {
                    r_next.set(GameState::Playing);
                    load_message.write(LoadGame {
                        name: "test1".to_string(),
                    });
                }),

                @FeathersButton {
                    @caption: bsn! { Text("Settings") ThemedText }
                }
                on(|_: On<Activate>, mut r_next: ResMut<NextState<MenuUiState>>| {
                    r_next.set(MenuUiState::Settings);
                }),

                @FeathersButton {
                    @caption: bsn! { Text("Quit") ThemedText }
                }
                on(|_: On<Activate>, mut exit: MessageWriter<AppExit>| {
                    exit.write(AppExit::Success);
                }),
            ]
        ]
    }
}

fn pause_menu() -> impl Scene {
    bsn! {
        Node {
            flex_direction: FlexDirection::Column,
            width: Val::Px(200.0),
        }
        Children [
            subpane_header() Children [
                (Text("Paused") ThemedText),
            ],
            subpane_body()
            Node {
                row_gap: px(8),
            }
            Children [
                @FeathersButton {
                    @caption: bsn! { Text("Resume") ThemedText }
                }
                on(|_: On<Activate>, mut r_next: ResMut<NextState<GameState>>| {
                    r_next.set(GameState::Playing);
                }),

                @FeathersButton {
                    @caption: bsn! { Text("Save") ThemedText }
                }
                on(|_: On<Activate>, mut r_next: ResMut<NextState<GameState>>, mut save_message: MessageWriter<SaveGame>| {
                    r_next.set(GameState::Playing);
                    save_message.write(SaveGame {
                        name: "test1".to_string(),
                    });
                }),

                @FeathersButton {
                    @caption: bsn! { Text("Settings") ThemedText }
                }
                on(|_: On<Activate>, mut r_next: ResMut<NextState<PauseUiState>>| {
                    r_next.set(PauseUiState::Settings);
                }),

                @FeathersButton {
                    @caption: bsn! { Text("Main Menu") ThemedText }
                }
                on(|_: On<Activate>, mut r_next: ResMut<NextState<GameState>>| {
                    r_next.set(GameState::MainMenu);
                }),
            ]
        ]
    }
}

fn settings_menu(back_func: impl Scene) -> impl Scene {
    bsn! {
        Node {
            flex_direction: FlexDirection::Column,
            width: Val::Px(200.0),
        }
        Children [
            subpane_header() Children [
                (Text("Settings") ThemedText),
            ],
            subpane_body()
            Node {
                row_gap: px(8),
            }
            Children [
                @FeathersButton {
                    @caption: bsn! { Text("Something 1") ThemedText }
                },

                @FeathersButton {
                    @caption: bsn! { Text("Something 2") ThemedText }
                },

                @FeathersButton {
                    @caption: bsn! { Text("Back") ThemedText }
                }
                { back_func },
            ]
        ]
    }
}
