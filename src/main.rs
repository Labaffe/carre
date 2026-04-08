use bevy::prelude::*;

mod asteroid;
mod background;
mod crosshair;
mod player;

use asteroid::AsteroidPlugin;
use background::BackgroundPlugin;
use crosshair::CrosshairPlugin;
use player::PlayerPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((BackgroundPlugin, CrosshairPlugin, PlayerPlugin, AsteroidPlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, mut windows: Query<&mut Window>, asset_server: Res<AssetServer>) {
    // masquer le curseur système
    windows.single_mut().cursor.visible = false;

    // caméra
    commands.spawn(Camera2dBundle::default());

    // musique de fond en boucle
    commands.spawn(AudioBundle {
        source: asset_server.load("gradius.ogg"),
        settings: PlaybackSettings::LOOP,
    });
}
