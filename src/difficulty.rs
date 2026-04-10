//! Système de difficulté progressive.
//!
//! - 0-10s  : facteur 1.0, montée en tension (son charging à 7s, boom à 10s)
//! - 10-20s : facteur 3.0 → 5.0 (augmente de +1 toutes les 5s)
//! - Le facteur influence : vitesse des astéroïdes, fréquence de spawn, scroll du background.

use crate::state::GameState;
use bevy::prelude::*;

pub struct DifficultyPlugin;

impl Plugin for DifficultyPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Difficulty::default())
            .add_systems(OnEnter(GameState::Playing), reset_difficulty)
            .add_systems(
                Update,
                update_difficulty.run_if(in_state(GameState::Playing)),
            );
    }
}

#[derive(Resource)]
pub struct Difficulty {
    pub elapsed: f32,
    pub factor: f32,
    pub charging_played: bool,
    pub boom_played: bool,
    pub boom_14_played: bool,
    pub boom_18_played: bool,
    pub boom_22_played: bool,
    pub boss_music_played: bool,
    /// Instant (elapsed) où la musique boss a été lancée.
    pub boss_music_start_time: Option<f32>,
    /// À partir de 26.7s, les astéroïdes ne spawnent plus.
    pub spawning_stopped: bool,
    /// Vitesse du background indépendante de la difficulté après 26.7s.
    /// None = utilise le calcul basé sur factor. Some(v) = vitesse fixe décroissante.
    pub bg_speed_override: Option<f32>,
    /// La grille 3×3 du background boss a été initialisée.
    pub boss_bg_initialized: bool,
}

impl Default for Difficulty {
    fn default() -> Self {
        Self {
            elapsed: 0.0,
            factor: 1.0,
            charging_played: false,
            boom_played: false,
            boom_14_played: false,
            boom_18_played: false,
            boom_22_played: false,
            boss_music_played: false,
            boss_music_start_time: None,
            spawning_stopped: false,
            bg_speed_override: None,
            boss_bg_initialized: false,
        }
    }
}

impl Difficulty {
    /// Intervalle entre deux spawns d'astéroïdes (en secondes), min 0.15s.
    pub fn spawn_interval(&self) -> f32 {
        (1.0 / self.factor).max(0.15)
    }
}

/// Temps à partir duquel les astéroïdes ne spawnent plus.
pub const SPAWN_STOP_TIME: f32 = 27.7;
/// Durée de décélération du background après SPAWN_STOP_TIME (en secondes).
const BG_DECEL_DURATION: f32 = 9.0;
/// Vitesse finale du background après décélération.
const BG_FINAL_SPEED: f32 = 30.0;

fn reset_difficulty(mut difficulty: ResMut<Difficulty>) {
    *difficulty = Difficulty::default();
}

//
fn update_difficulty(
    mut difficulty: ResMut<Difficulty>,
    time: Res<Time>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    difficulty.elapsed += time.delta_seconds();

    // Son charging à 7s
    if difficulty.elapsed >= 7.0 && !difficulty.charging_played {
        difficulty.charging_played = true;
        commands.spawn(AudioBundle {
            source: asset_server.load("audio/charging.ogg"),
            settings: PlaybackSettings::DESPAWN,
        });
    }

    // Son boom à 10s
    if difficulty.elapsed >= 10.0 && !difficulty.boom_played {
        difficulty.boom_played = true;
        commands.spawn(AudioBundle {
            source: asset_server.load("audio/boom.wav"),
            settings: PlaybackSettings::DESPAWN,
        });
    }

    // Son boom à 14.3s
    if difficulty.elapsed >= 14.3 && !difficulty.boom_14_played {
        difficulty.boom_14_played = true;
        commands.spawn(AudioBundle {
            source: asset_server.load("audio/boom.wav"),
            settings: PlaybackSettings::DESPAWN,
        });
    }

    // Son boom à 18.3s
    if difficulty.elapsed >= 18.3 && !difficulty.boom_18_played {
        difficulty.boom_18_played = true;
        commands.spawn(AudioBundle {
            source: asset_server.load("audio/boom.wav"),
            settings: PlaybackSettings::DESPAWN,
        });
    }

    // Son boom à 22.6s
    if difficulty.elapsed >= 22.6 && !difficulty.boom_22_played {
        difficulty.boom_22_played = true;
        commands.spawn(AudioBundle {
            source: asset_server.load("audio/boom.wav"),
            settings: PlaybackSettings::DESPAWN,
        });
    }

    // Paliers de difficulté fixes :
    // 0-10s    : facteur 1.0
    // 10s      : facteur 3.0
    // 14.3s    : facteur 5.0
    // 18.3s    : facteur 7.0
    // 22.6s    : facteur 9.0 (max)
    if difficulty.elapsed < 10.0 {
        difficulty.factor = 1.0;
    } else if difficulty.elapsed < 14.3 {
        difficulty.factor = 3.0;
    } else if difficulty.elapsed < 18.3 {
        difficulty.factor = 4.0;
    } else if difficulty.elapsed < 22.6 {
        difficulty.factor = 6.0;
    } else {
        difficulty.factor = 7.0;
    }

    // À 26.7s : arrêt du spawn + décélération du background vers 50 px/s en 6s
    if difficulty.elapsed >= SPAWN_STOP_TIME {
        difficulty.spawning_stopped = true;

        let decel_elapsed = difficulty.elapsed - SPAWN_STOP_TIME;
        let t = (decel_elapsed / BG_DECEL_DURATION).clamp(0.0, 1.0);
        // Vitesse du background au moment de l'arrêt (basée sur la formule du background)
        let bg_speed_at_stop = 150.0 * (1.0 + 8.0 * 3.0); // base_speed * (1 + factor * 3)
        let current_speed = bg_speed_at_stop + (BG_FINAL_SPEED - bg_speed_at_stop) * t;
        difficulty.bg_speed_override = Some(current_speed);
    }

    // Note : la musique boss est lancée par boss.rs à la fin de l'animation d'entrée.
}
