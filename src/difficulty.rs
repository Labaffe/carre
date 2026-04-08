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
}

impl Default for Difficulty {
    fn default() -> Self {
        Self {
            elapsed: 0.0,
            factor: 1.0,
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
fn update_difficulty(mut difficulty: ResMut<Difficulty>, time: Res<Time>) {
    difficulty.elapsed += time.delta_seconds();
    // 0-10s : facteur 1.0 fixe
    // après 10s : +1.0 toutes les 5 secondes (10s→2, 15s→3, 20s→4…)
    if difficulty.elapsed <= 10.0 {
        difficulty.factor = 1.0;
    } else if difficulty.elapsed <= 20.0 {
        difficulty.factor = 2.0 + (difficulty.elapsed - 10.0) / 5.0;
    }
}
