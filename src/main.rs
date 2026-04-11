use bevy::prelude::*;

mod asteroid;
mod background;
mod boss;
mod collision;
mod countdown;
mod crosshair;
mod debug;
mod difficulty;
pub mod enemies;
pub mod enemy;
mod explosion;
pub mod game;
mod gameover;
mod green_ufo;
mod level;
mod levelselect;
pub mod item;
mod mainmenu;
mod missile;
pub mod pause;
mod player;
mod state;
mod weapon;
mod score;

use asteroid::{Asteroid, AsteroidPlugin};
use background::{Background, BackgroundPlugin, Planet};
use boss::{BossPlugin, MusicBoss};
use game::{GamePlugin, MusicOutro};
use collision::CollisionPlugin;
use countdown::CountdownPlugin;
use crosshair::CrosshairPlugin;
use debug::DebugPlugin;
use difficulty::DifficultyPlugin;
use enemy::{Enemy, EnemyPlugin, EnemyProjectile};
use explosion::{Explosion, ExplosionPlugin};
use item::{Droppable, ItemPlugin};
use gameover::GameOverPlugin;
use level::LevelPlugin;
use mainmenu::MainMenuPlugin;
use missile::{Missile, MissilePlugin};
use pause::PausePlugin;
use player::{Player, PlayerPlugin};
use state::GameState;
use weapon::WeaponPlugin;
use score::ScorePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Carré".to_string(),
                mode: bevy::window::WindowMode::BorderlessFullscreen,
                visible: false,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::BLACK))
        .init_resource::<GameSettings>()
        .init_state::<GameState>()
        .add_plugins((
            BackgroundPlugin,
            CrosshairPlugin,
            PlayerPlugin,
            AsteroidPlugin,
            CollisionPlugin,
            CountdownPlugin,
            GameOverPlugin,
            MainMenuPlugin,
            DebugPlugin,
            DifficultyPlugin,
            MissilePlugin,
            ExplosionPlugin,
            WeaponPlugin,
            PausePlugin,
            ScorePlugin
        ))
        .add_plugins((
            EnemyPlugin,
            BossPlugin,
            ItemPlugin,
            green_ufo::GreenUFOPlugin,
            LevelPlugin,
            GamePlugin,
            levelselect::LevelSelectPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, show_window_after_render.run_if(run_once()))
        .add_systems(OnExit(GameState::Playing), cleanup_playing)
        .run();
}

/// Volume global du jeu (0.0 – 1.0).
#[derive(Resource)]
pub struct GameSettings {
    pub master_volume: f32,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self { master_volume: 0.3 }
    }
}

#[derive(Component)]
pub struct MusicMain;

#[derive(Component)]
pub struct MusicGameOver;

fn setup(mut commands: Commands, settings: Res<GameSettings>) {
    commands.spawn(Camera2dBundle::default());
    commands.insert_resource(GlobalVolume {
        volume: bevy::audio::Volume::new(settings.master_volume),
    });
}

/// Affiche la fenêtre après la première frame (évite le flash blanc Windows).
fn show_window_after_render(mut windows: Query<&mut Window>) {
    windows.single_mut().visible = true;
}

/// Nettoyage de toutes les entités de jeu quand on quitte l'état Playing.
fn cleanup_playing(
    mut commands: Commands,
    players: Query<Entity, With<Player>>,
    asteroids: Query<Entity, With<Asteroid>>,
    missiles: Query<Entity, With<Missile>>,
    explosions: Query<Entity, With<Explosion>>,
    backgrounds: Query<Entity, With<Background>>,
    planets: Query<Entity, With<Planet>>,
    enemies: Query<Entity, With<Enemy>>,
    enemy_projectiles: Query<Entity, With<EnemyProjectile>>,
    music: Query<Entity, With<MusicMain>>,
    boss_music: Query<Entity, With<MusicBoss>>,
    outro_music: Query<Entity, With<MusicOutro>>,
    droppables: Query<Entity, With<Droppable>>,
) {
    let all_entities = players.iter()
        .chain(asteroids.iter())
        .chain(missiles.iter())
        .chain(explosions.iter())
        .chain(backgrounds.iter())
        .chain(planets.iter())
        .chain(enemies.iter())
        .chain(enemy_projectiles.iter())
        .chain(music.iter())
        .chain(boss_music.iter())
        .chain(outro_music.iter())
        .chain(droppables.iter());

    for entity in all_entities {
        if let Some(e) = commands.get_entity(entity) {
            e.despawn_recursive();
        }
    }
}

