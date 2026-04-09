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
        }
    }
}

impl Difficulty {
    /// Intervalle entre deux spawns d'astéroïdes (en secondes), min 0.15s.
    pub fn spawn_interval(&self) -> f32 {
        (1.0 / self.factor).max(0.15)
    }
}

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
        difficulty.factor = 5.0;
    } else if difficulty.elapsed < 22.6 {
        difficulty.factor = 7.0;
    } else {
        difficulty.factor = 8.0;
    }
}
