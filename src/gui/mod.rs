use crate::gui::game::GameUiPlugin;
use crate::gui::menu::MenuPlugin;
use bevy::asset::{embedded_asset, load_embedded_asset};
use bevy::feathers::FeathersPlugins;
use bevy::feathers::dark_theme::create_dark_theme;
use bevy::feathers::theme::UiTheme;
use bevy::prelude::*;

mod game;
pub mod menu;

pub struct GuiPlugin;

impl Plugin for GuiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FeathersPlugins);
        app.insert_resource(UiTheme(create_dark_theme()));

        embedded_asset!(app, "../../assets/fonts/Roboto/Roboto.ttf");
        embedded_asset!(app, "../../assets/fonts/Roboto/Roboto-Italic.ttf");

        app.add_plugins(MenuPlugin);
        app.add_plugins(GameUiPlugin);
    }
}

#[derive(Resource)]
struct UiFonts {
    regular: Handle<Font>,
    italic: Handle<Font>,
}

fn setup_fonts(mut commands: Commands, asset_server: Res<AssetServer>) {
    let regular = load_embedded_asset!(&*asset_server, "Roboto.ttf");
    let italic = load_embedded_asset!(&*asset_server, "Roboto-Italic.ttf");

    commands.insert_resource(UiFonts { regular, italic });
}
