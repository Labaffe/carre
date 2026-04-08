use bevy::prelude::*;

mod asteroid;
mod background;
mod collision;
mod crosshair;
mod gameover;
mod player;
mod state;

use asteroid::AsteroidPlugin;
use background::BackgroundPlugin;
use collision::CollisionPlugin;
use crosshair::CrosshairPlugin;
use gameover::GameOverPlugin;
use player::PlayerPlugin;
use state::GameState;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<GameState>()
        .add_plugins((
            BackgroundPlugin,
            CrosshairPlugin,
            PlayerPlugin,
            AsteroidPlugin,
            CollisionPlugin,
            GameOverPlugin,
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, mut windows: Query<&mut Window>, asset_server: Res<AssetServer>) {
    windows.single_mut().cursor.visible = false;

    commands.spawn(Camera2dBundle::default());

    commands.spawn(AudioBundle {
        source: asset_server.load("gradius.ogg"),
        settings: PlaybackSettings::LOOP,
    });
}
