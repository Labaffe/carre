use bevy::prelude::*;

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
pub mod pause;
mod player;
mod state;
mod weapon;

use asteroid::{Asteroid, AsteroidPlugin};
use background::{Background, BackgroundPlugin};
use collision::CollisionPlugin;
use crosshair::CrosshairPlugin;
use debug::DebugPlugin;
use difficulty::DifficultyPlugin;
use explosion::{Explosion, ExplosionPlugin};
use gameover::GameOverPlugin;
use mainmenu::MainMenuPlugin;
use missile::{Missile, MissilePlugin};
use pause::PausePlugin;
use player::{Player, PlayerPlugin};
use state::GameState;
use weapon::WeaponPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Carré".to_string(),
                mode: bevy::window::WindowMode::BorderlessFullscreen,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::BLACK))
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
            PausePlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(OnEnter(GameState::Playing), start_game_music)
        .add_systems(OnExit(GameState::Playing), cleanup_playing)
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

/// Nettoyage de toutes les entités de jeu quand on quitte l'état Playing.
fn cleanup_playing(
    mut commands: Commands,
    players: Query<Entity, With<Player>>,
    asteroids: Query<Entity, With<Asteroid>>,
    missiles: Query<Entity, With<Missile>>,
    explosions: Query<Entity, With<Explosion>>,
    backgrounds: Query<Entity, With<Background>>,
    music: Query<Entity, With<MusicMain>>,
) {
    for entity in players.iter() {
        commands.entity(entity).despawn_recursive();
    }
    for entity in asteroids.iter() {
        commands.entity(entity).despawn_recursive();
    }
    for entity in missiles.iter() {
        commands.entity(entity).despawn_recursive();
    }
    for entity in explosions.iter() {
        commands.entity(entity).despawn_recursive();
    }
    for entity in backgrounds.iter() {
        commands.entity(entity).despawn_recursive();
    }
    for entity in music.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

/// Spawn la musique de jeu en boucle.
pub fn spawn_main_music(commands: &mut Commands, asset_server: &Res<AssetServer>) {
    commands.spawn((
        AudioBundle {
            source: asset_server.load("audio/gradius.ogg"),
            settings: PlaybackSettings {
                mode: bevy::audio::PlaybackMode::Loop,
                volume: bevy::audio::Volume::new(0.6),
                ..default()
            },
        },
        MusicMain,
    ));
}
