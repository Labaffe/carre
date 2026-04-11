//! Système de difficulté progressive.
//!
//! La ressource `Difficulty` est le hub central de communication entre
//! le système de niveau (`level.rs`) et les systèmes de jeu (astéroïdes,
//! boss, background, etc.).
//!
//! Le système de niveau écrit les valeurs (factor, spawning_stopped, etc.)
//! et les systèmes de jeu les lisent pour adapter leur comportement.

use std::collections::HashMap;

use crate::countdown::CountdownEvent;
use crate::pause::not_paused;
use crate::state::GameState;
use bevy::prelude::*;

/// Événement envoyé à chaque boom (palier de difficulté).
#[derive(Event)]
pub struct BoomEvent;

pub struct DifficultyPlugin;

impl Plugin for DifficultyPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Difficulty::default())
            .add_event::<BoomEvent>()
            .add_systems(OnEnter(GameState::Playing), reset_difficulty)
            .add_systems(
                Update,
                update_difficulty
                    .run_if(in_state(GameState::Playing))
                    .run_if(not_paused),
            );
    }
}

#[derive(Resource)]
pub struct Difficulty {
    pub elapsed: f32,
    pub factor: f32,
    pub boss_music_played: bool,
    /// Instant (elapsed) où la musique boss a été lancée.
    pub boss_music_start_time: Option<f32>,
    /// Instant (elapsed) où le boss est passé en Active (fin du flexing).
    pub boss_active_time: Option<f32>,
    /// Les astéroïdes ne spawnent plus.
    pub spawning_stopped: bool,
    /// Vitesse du background indépendante de la difficulté.
    /// None = utilise le calcul basé sur factor. Some(v) = vitesse fixe décroissante.
    pub bg_speed_override: Option<f32>,
    /// La grille 3×3 du background boss a été initialisée.
    pub boss_bg_initialized: bool,
    /// Son landing.ogg joué (5s avant la fin de PLANET_ANIM_DURATION).
    pub landing_played: bool,
    /// Le boss a déjà été spawné (empêche le double spawn avec F3).
    pub boss_spawned: bool,
    /// Son charging joué avant la phase 3 du vaisseau.
    pub phase3_charging_played: bool,
    /// Son boom joué au passage en phase 3 du vaisseau.
    pub phase3_boom_played: bool,

    // ─── Communication Level → systèmes de jeu ─────────────────
    /// File de requêtes de spawn one-shot : (nom, quantité).
    /// Ex: `("boss", 2)` spawne 2 boss, `("green_ufo", 4)` spawne 4 GreenUFO.
    /// Consommées par le système de spawn de chaque ennemi.
    pub spawn_requests: Vec<(&'static str, usize)>,
    /// Spawners continus actifs : nom → (quantité par vague, intervalle en secondes).
    /// Ex: `"green_ufo" → (4, 5.0)` spawne 4 GreenUFOs toutes les 5s.
    pub active_spawners: HashMap<&'static str, (usize, f32)>,
    /// Instant (elapsed) où la décélération du background a commencé.
    pub bg_decel_start_elapsed: Option<f32>,
    /// Durée de la décélération du background (secondes).
    pub bg_decel_duration: f32,
    /// Vitesse finale du background après décélération.
    pub bg_decel_final_speed: f32,
    /// Instant (elapsed) où la planète doit apparaître.
    pub planet_appear_elapsed: Option<f32>,
}

impl Default for Difficulty {
    fn default() -> Self {
        Self {
            elapsed: 0.0,
            factor: 1.0,
            boss_music_played: false,
            boss_music_start_time: None,
            boss_active_time: None,
            spawning_stopped: false,
            bg_speed_override: None,
            boss_bg_initialized: false,
            landing_played: false,
            boss_spawned: false,
            phase3_charging_played: false,
            phase3_boom_played: false,
            spawn_requests: Vec::new(),
            active_spawners: HashMap::new(),
            bg_decel_start_elapsed: None,
            bg_decel_duration: 9.0,
            bg_decel_final_speed: 30.0,
            planet_appear_elapsed: None,
        }
    }
}

impl Difficulty {
    /// Intervalle entre deux spawns d'astéroïdes (en secondes), min 0.15s.
    pub fn spawn_interval(&self) -> f32 {
        (1.0 / self.factor).max(0.15)
    }
}

/// Temps à partir duquel les astéroïdes ne spawnent plus (utilisé par F2 debug).
pub const SPAWN_STOP_TIME: f32 = 27.7;

fn reset_difficulty(mut difficulty: ResMut<Difficulty>) {
    *difficulty = Difficulty::default();
}

/// Met à jour la difficulté chaque frame.
/// Les événements temporels sont maintenant gérés par `level.rs`.
/// Ce système gère uniquement :
/// - L'incrément du timer
/// - Le countdown de la phase 3 (déclenché par la musique boss)
/// - La décélération du background (déclenchée par le niveau)
fn update_difficulty(
    mut difficulty: ResMut<Difficulty>,
    time: Res<Time>,
    mut countdown_events: EventWriter<CountdownEvent>,
) {
    difficulty.elapsed += time.delta_seconds();

    // Countdown phase 3 : dès que la musique boss démarre
    if let Some(_start) = difficulty.boss_music_start_time {
        if !difficulty.phase3_charging_played {
            difficulty.phase3_charging_played = true;
            difficulty.phase3_boom_played = true;
            countdown_events.send(CountdownEvent);
        }
    }

    // Décélération du background (déclenchée par le niveau via StartBgDeceleration)
    if let Some(decel_start) = difficulty.bg_decel_start_elapsed {
        let decel_elapsed = difficulty.elapsed - decel_start;
        let t = (decel_elapsed / difficulty.bg_decel_duration).clamp(0.0, 1.0);
        let bg_speed_at_stop = 150.0 * (1.0 + 8.0 * 3.0);
        let current_speed = bg_speed_at_stop
            + (difficulty.bg_decel_final_speed - bg_speed_at_stop) * t;
        difficulty.bg_speed_override = Some(current_speed);
    }
}
