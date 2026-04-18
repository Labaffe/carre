use bevy::prelude::*;

// ─── Core ──────────────────────────────────────────────────────────
mod state;
mod difficulty;
mod level;
pub mod game;

// ─── Joueur & armes ────────────────────────────────────────────────
mod player;
mod missile;
mod weapon;
mod crosshair;
mod collision;

// ─── Ennemis ───────────────────────────────────────────────────────
pub mod enemy;
pub mod enemies;
mod boss;
mod green_ufo;

// ─── Entités & effets ──────────────────────────────────────────────
mod asteroid;
mod explosion;
pub mod item;

// ─── UI & écrans ───────────────────────────────────────────────────
mod mainmenu;
mod levelselect;
mod gameover;
pub mod pause;
mod countdown;
mod score;
mod deckbuilding;
mod tweening;
// ─── Rendu & debug ─────────────────────────────────────────────────
mod background;
mod debug;

// ─── Imports ───────────────────────────────────────────────────────
use state::GameState;
use game::{GamePlugin, MusicOutro};
use difficulty::DifficultyPlugin;
use level::LevelPlugin;

use player::{Player, PlayerPlugin};
use missile::{Missile, MissilePlugin};
use weapon::WeaponPlugin;
use crosshair::CrosshairPlugin;
use collision::CollisionPlugin;

use enemy::{Enemy, EnemyPlugin, EnemyProjectile};
use boss::{BossPlugin, MusicBoss};

use asteroid::{Asteroid, AsteroidPlugin};
use explosion::{Explosion, ExplosionPlugin};
use item::{Droppable, ItemPlugin};

use mainmenu::MainMenuPlugin;
use gameover::GameOverPlugin;
use pause::PausePlugin;
use countdown::CountdownPlugin;
use score::ScorePlugin;

use background::{Background, BackgroundPlugin, Planet};
use debug::DebugPlugin;

use deckbuilding::card_hand::CardHandPlugin;
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
        // Core
        .add_plugins((
            DifficultyPlugin,
            LevelPlugin,
            GamePlugin,
        ))
        // Joueur & armes
        .add_plugins((
            PlayerPlugin,
            MissilePlugin,
            WeaponPlugin,
            CrosshairPlugin,
            CollisionPlugin,
        ))
        // Ennemis
        .add_plugins((
            EnemyPlugin,
            BossPlugin,
            green_ufo::GreenUFOPlugin,
        ))
        // Entités & effets
        .add_plugins((
            AsteroidPlugin,
            ExplosionPlugin,
            ItemPlugin,
        ))
        // UI & écrans
        .add_plugins((
            MainMenuPlugin,
            levelselect::LevelSelectPlugin,
            GameOverPlugin,
            PausePlugin,
            CardHandPlugin,
            CountdownPlugin,
            ScorePlugin,
            tweening::plugin::UiTweenPlugin,
        ))
        // Rendu & debug
        .add_plugins((
            BackgroundPlugin,
            DebugPlugin,
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

