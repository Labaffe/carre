use bevy::prelude::*;
use bevy::window::WindowMode;

mod asteroid;
mod background;
mod collision;
mod crosshair;
mod debug;
mod difficulty;
mod explosion;
mod gameover;
mod mainmenu;
mod missile;
mod player;
mod state;
mod weapon;

use asteroid::AsteroidPlugin;
use background::BackgroundPlugin;
use collision::CollisionPlugin;
use crosshair::CrosshairPlugin;
use debug::DebugPlugin;
use difficulty::DifficultyPlugin;
use explosion::ExplosionPlugin;
use gameover::GameOverPlugin;
use mainmenu::MainMenuPlugin;
use missile::MissilePlugin;
use player::PlayerPlugin;
use state::GameState;
use weapon::WeaponPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Carré".to_string(),
                mode: WindowMode::BorderlessFullscreen,
                ..default()
            }),
            ..default()
        }))
        .init_state::<GameState>()
        .add_plugins((
            BackgroundPlugin,
            CrosshairPlugin,
            PlayerPlugin,
            AsteroidPlugin,
            CollisionPlugin,
            GameOverPlugin,
            MainMenuPlugin,
            DebugPlugin,
            DifficultyPlugin,
            MissilePlugin,
            ExplosionPlugin,
            WeaponPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(OnEnter(GameState::Playing), start_game_music)
        .run();
}

#[derive(Component)]
pub struct MusicMain;

#[derive(Component)]
pub struct MusicGameOver;

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

/// Système lancé à chaque entrée en Playing : démarre la musique de jeu.
fn start_game_music(mut commands: Commands, asset_server: Res<AssetServer>) {
    spawn_main_music(&mut commands, &asset_server);
}

/// Spawn la musique de jeu en boucle.
pub fn spawn_main_music(commands: &mut Commands, asset_server: &Res<AssetServer>) {
    commands.spawn((
        AudioBundle {
            source: asset_server.load("audio/gradius.ogg"),
            settings: PlaybackSettings {
                mode: bevy::audio::PlaybackMode::Loop,
                volume: bevy::audio::Volume::new(0.7),
                ..default()
            },
        },
        MusicMain,
    ));
}
