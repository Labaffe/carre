use bevy::prelude::*;

mod asteroid;
mod background;
mod collision;
mod crosshair;
mod debug;
mod difficulty;
mod gameover;
mod player;
mod state;
mod thruster;

use asteroid::AsteroidPlugin;
use background::BackgroundPlugin;
use collision::CollisionPlugin;
use crosshair::CrosshairPlugin;
use debug::DebugPlugin;
use difficulty::DifficultyPlugin;
use gameover::GameOverPlugin;
use player::PlayerPlugin;
use state::GameState;
use thruster::ThrusterPlugin;

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
            DebugPlugin,
            DifficultyPlugin,
            ThrusterPlugin,
        ))
        .add_systems(Startup, setup)
        .run();
}

#[derive(Component)]
pub struct MusicMain;

#[derive(Component)]
pub struct MusicGameOver;

fn setup(mut commands: Commands, mut windows: Query<&mut Window>, asset_server: Res<AssetServer>) {
    windows.single_mut().cursor.visible = false;

    commands.spawn(Camera2dBundle::default());

    spawn_main_music(&mut commands, &asset_server);
}

pub fn spawn_main_music(commands: &mut Commands, asset_server: &Res<AssetServer>) {
    commands.spawn((
        AudioBundle {
            source: asset_server.load("gradius.ogg"),
            settings: PlaybackSettings::LOOP,
        },
        MusicMain,
    ));
}
