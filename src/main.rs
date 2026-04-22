use bevy::prelude::*;

// ─── Modules par feature ──────────────────────────────────────────
mod debug;
mod deckbuilding;
mod enemy;
mod environment;
mod fx;
mod game_manager;
mod item;
mod level;
mod menu;
mod physic;
mod player;
mod tweening;
mod ui;
mod weapon;

// ─── Imports ───────────────────────────────────────────────────────
use game_manager::state::GameState;
use game_manager::game::{GamePlugin, MusicOutro};
use game_manager::difficulty::DifficultyPlugin;

use level::level::{LevelConfig, LevelPlugin};

use player::player::{Player, PlayerPlugin};
use weapon::weapon::WeaponPlugin;
use weapon::player_fire::PlayerFirePlugin;
use weapon::projectile::{Projectile, ProjectilePlugin};

use enemy::enemy::{Enemy, EnemyPlugin};
use enemy::boss::{BossPlugin, MusicBoss};
use enemy::asteroid::{Asteroid, AsteroidPlugin};
use enemy::green_ufo::GreenUFOPlugin;
use enemy::gatling::GatlingPlugin;
use enemy::mothership::{GatlingLaser, MothershipMarker};

use fx::explosion::{Explosion, ExplosionPlugin};
use item::item::{Droppable, ItemPlugin};

use menu::mainmenu::MainMenuPlugin;
use menu::pause::PausePlugin;
use menu::gameover::GameOverPlugin;
use menu::levelselect::LevelSelectPlugin;

use ui::crosshair::CrosshairPlugin;
use ui::score::ScorePlugin;
use ui::countdown::CountdownPlugin;

use environment::background::{Background, BackgroundPlugin, Planet};
use physic::collision::CollisionPlugin;

use debug::debug::DebugPlugin;
use deckbuilding::DeckbuildingPlugin;
use tweening::plugin::UiTweenPlugin;
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
        .init_resource::<LevelConfig>()
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
            PlayerFirePlugin,
            WeaponPlugin,
            ProjectilePlugin,
            CrosshairPlugin,
            CollisionPlugin,
        ))
        // Ennemis
        .add_plugins((
            EnemyPlugin,
            BossPlugin,
            GreenUFOPlugin,
            GatlingPlugin,
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
            LevelSelectPlugin,
            GameOverPlugin,
            PausePlugin,
            DeckbuildingPlugin,
            CountdownPlugin,
            ScorePlugin,
            UiTweenPlugin,
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
    projectiles: Query<Entity, With<Projectile>>,
    explosions: Query<Entity, With<Explosion>>,
    backgrounds: Query<Entity, With<Background>>,
    planets: Query<Entity, With<Planet>>,
    enemies: Query<Entity, With<Enemy>>,
    music: Query<Entity, With<MusicMain>>,
    boss_music: Query<Entity, With<MusicBoss>>,
    outro_music: Query<Entity, With<MusicOutro>>,
    droppables: Query<Entity, With<Droppable>>,
    motherships: Query<Entity, With<MothershipMarker>>,
    lasers: Query<Entity, With<GatlingLaser>>,
) {
    let all_entities = players.iter()
        .chain(asteroids.iter())
        .chain(projectiles.iter())
        .chain(explosions.iter())
        .chain(backgrounds.iter())
        .chain(planets.iter())
        .chain(enemies.iter())
        .chain(music.iter())
        .chain(boss_music.iter())
        .chain(outro_music.iter())
        .chain(droppables.iter())
        .chain(motherships.iter())
        .chain(lasers.iter());

    for entity in all_entities {
        if let Some(e) = commands.get_entity(entity) {
            e.despawn_recursive();
        }
    }
}

