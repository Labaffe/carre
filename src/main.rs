use bevy::prelude::*;

// ─── Modules par feature ──────────────────────────────────────────
mod audio;
mod debug;
mod deckbuilding;
mod enemy;
mod behavior;
mod sprite_orient;
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
mod editor;
mod movement;
mod geometry;
// ─── Imports ───────────────────────────────────────────────────────
use game_manager::state::GameState;
use game_manager::game::GamePlugin;
use game_manager::difficulty::DifficultyPlugin;
use game_manager::level_up::LevelUpPlugin;

use editor::EditorPlugin;
use behavior::BehaviorPlugin;

use level::level::{LevelConfig, LevelPlugin};

use player::player::PlayerPlugin;
use weapon::weapon::WeaponPlugin;
use weapon::player_fire::PlayerFirePlugin;
use weapon::projectile::ProjectilePlugin;

use enemy::EnemyPlugin;

use crate::movement::despawn_off_screen::DespawnOffScreenPlugin;
use fx::explosion::ExplosionPlugin;
use fx::screen_shake::ScreenShakePlugin;
use fx::time_fx::TimeFxPlugin;
use item::item::ItemPlugin;

use menu::mainmenu::MainMenuPlugin;
use menu::pause::PausePlugin;
use menu::gameover::GameOverPlugin;
use menu::levelselect::LevelSelectPlugin;

use ui::crosshair::CrosshairPlugin;
use ui::score::ScorePlugin;
use ui::countdown::CountdownPlugin;
use ui::stats::StatsUiPlugin;
use ui::xp_bar::XpBarPlugin;

use environment::background::BackgroundPlugin;
use physic::collider::ColliderPlugin;
use physic::collision::CollisionPlugin;
use physic::player_detection::PlayerDetectionPlugin;

use debug::debug::DebugPlugin;
use deckbuilding::DeckbuildingPlugin;
use tweening::plugin::UiTweenPlugin;

use movement::MovementPlugin;
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Carré".to_string(),
                mode: bevy::window::WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
                visible: true,
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
            game_manager::loading::LoadingPlugin,
            audio::AudioPlugin,
            sprite_orient::SpriteOrientPlugin,
        ))
        .add_plugins(
            EditorPlugin
            
        )
        .add_plugins(
            BehaviorPlugin
        )
        .add_plugins(
            MovementPlugin
        )
        // Joueur & armes
        .add_plugins((
            PlayerPlugin,
            player::power::PowerPlugin,
            player::shield::ShieldPlugin,
            PlayerFirePlugin,
            WeaponPlugin,
            ProjectilePlugin,
            CrosshairPlugin,
            CollisionPlugin,
            ColliderPlugin,
            physic::health::HealthPlugin,
            physic::no_overlap::NoOverlapPlugin,
            PlayerDetectionPlugin,
        ))
        // Ennemis
        .add_plugins((
            EnemyPlugin,
            DespawnOffScreenPlugin
        ))
        // Entités & effets
        .add_plugins((
            ExplosionPlugin,
            ScreenShakePlugin,
            TimeFxPlugin,
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
            LevelUpPlugin,
            StatsUiPlugin,
            XpBarPlugin,
        ))
        // Rendu & debug
        .add_plugins((
            BackgroundPlugin,
            DebugPlugin,
        ))
        .add_systems(Startup, setup)
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

/// Marker apposé sur **toutes les entités à durée de vie = état `Playing`** :
/// entités gameplay (Player, Enemy, Projectile, AOE, Background, Planet,
/// Explosion, Droppable, Music...) ET UI gameplay (ScoreUI, LivesUI, BombUI,
/// ChaosLevelUI, PowerUIRoot, WeaponUI...). Tout est despawn ensemble par
/// `cleanup_playing` quand on quitte `Playing`.
///
/// Ajouté automatiquement via `#[require(GameplayEntity)]` sur les composants
/// racines. Un seul query dans le cleanup, zéro système de cleanup par
/// module à maintenir.
///
/// Pour qu'un nouvel ennemi / UI / entity profite du cleanup auto : juste
/// `#[require(crate::GameplayEntity)]` sur son marker racine.
#[derive(Component, Default, Clone)]
pub struct GameplayEntity;

#[derive(Component)]
#[require(GameplayEntity)]
pub struct MusicMain;

#[derive(Component)]
pub struct MusicGameOver;

fn setup(mut commands: Commands, settings: Res<GameSettings>) {
    commands.spawn(Camera2d);
    commands.insert_resource(GlobalVolume {
        volume: bevy::audio::Volume::Linear(settings.master_volume),
    });
}

/// Nettoyage unifié : despawn toutes les entités portant `GameplayEntity`
/// quand on quitte l'état Playing. Le marker est ajouté automatiquement via
/// `#[require(GameplayEntity)]` sur les composants racines (voir leurs
/// définitions respectives).
fn cleanup_playing(
    mut commands: Commands,
    entities: Query<Entity, With<GameplayEntity>>,
) {
    for entity in entities.iter() {
        if let Ok(mut e) = commands.get_entity(entity) {
            e.try_despawn();
        }
    }
}

