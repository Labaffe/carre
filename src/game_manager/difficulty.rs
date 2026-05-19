//! Système de difficulté progressive.
//!
//! La ressource `Difficulty` est le hub central de communication entre
//! le système de niveau (`level.rs`) et les systèmes de jeu (astéroïdes,
//! boss, background, etc.).
//!
//! Le système de niveau écrit les valeurs (factor, spawning_stopped, etc.)
//! et les systèmes de jeu les lisent pour adapter leur comportement.

use std::collections::HashMap;

use crate::game_manager::state::GameState;
use crate::menu::pause::not_paused;
use bevy::prelude::*;

/// Position de spawn d'un ennemi.
#[derive(Clone, Copy, Debug)]
pub enum SpawnPosition {
    /// Position aléatoire sur le bord haut de l'écran (défaut pour les UFOs).
    Top,
    /// Position aléatoire sur le bord bas.
    Bottom,
    /// Position aléatoire sur le bord gauche.
    Left,
    /// Position aléatoire sur le bord droit.
    Right,
    /// Position exacte en pixels (x, y).
    At(f32, f32),
}

impl SpawnPosition {
    /// Résout la position de spawn en coordonnées monde.
    /// `margin` = marge intérieure par rapport au bord.
    pub fn resolve(self, window: &bevy::window::Window, margin: f32) -> bevy::math::Vec2 {
        let half_w = window.width() / 2.0 - margin;
        let half_h = window.height() / 2.0;
        match self {
            SpawnPosition::Top => {
                let x = (fastrand::f32() - 0.5) * 2.0 * half_w;
                bevy::math::Vec2::new(x, half_h + 40.0)
            }
            SpawnPosition::Bottom => {
                let x = (fastrand::f32() - 0.5) * 2.0 * half_w;
                bevy::math::Vec2::new(x, -half_h - 40.0)
            }
            SpawnPosition::Left => {
                let y = (fastrand::f32() - 0.5) * 2.0 * half_h;
                bevy::math::Vec2::new(-half_w - 40.0, y)
            }
            SpawnPosition::Right => {
                let y = (fastrand::f32() - 0.5) * 2.0 * half_h;
                bevy::math::Vec2::new(half_w + 40.0, y)
            }
            SpawnPosition::At(x, y) => bevy::math::Vec2::new(x, y),
        }
    }
}

impl Default for SpawnPosition {
    fn default() -> Self {
        SpawnPosition::Top
    }
}

/// Événement envoyé à chaque boom (palier de difficulté).
#[derive(Message)]
pub struct BoomEvent;

pub struct DifficultyPlugin;

impl Plugin for DifficultyPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Difficulty::default())
            .add_message::<BoomEvent>()
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
    /// Vitesse du background indépendante de la difficulté.
    /// None = utilise le calcul basé sur factor. Some(v) = vitesse fixe décroissante.
    pub bg_speed_override: Option<f32>,
    /// Son landing.ogg joué (5s avant la fin de PLANET_ANIM_DURATION).
    pub landing_played: bool,
    /// Le boss a déjà été spawné (empêche le double spawn avec F3).
    pub boss_spawned: bool,
    /// Un boss a été observé vivant dans une query (entité réellement présente).
    /// Sert à éviter la race condition : boss_spawned=true mais Commands pas encore appliquées.
    pub boss_seen_alive: bool,
    /// Le niveau est terminé — déclenche l'outro.
    pub level_complete: bool,

    // ─── Communication Level → systèmes de jeu ─────────────────
    /// File de requêtes de spawn one-shot : (nom, quantité, position).
    pub spawn_requests: Vec<(&'static str, usize, SpawnPosition)>,
    /// Spawners continus actifs : nom → (quantité par vague, intervalle, position).
    pub active_spawners: HashMap<&'static str, (usize, f32, SpawnPosition)>,
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
            bg_speed_override: None,
            landing_played: false,
            boss_spawned: false,
            boss_seen_alive: false,
            level_complete: false,
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

fn reset_difficulty(mut difficulty: ResMut<Difficulty>) {
    *difficulty = Difficulty::default();
}

/// Met à jour la difficulté chaque frame.
/// Les événements temporels sont gérés par `level.rs`.
/// Ce système gère uniquement :
/// - L'incrément du timer
/// - La décélération du background (déclenchée par le niveau)
fn update_difficulty(mut difficulty: ResMut<Difficulty>, time: Res<Time>) {
    difficulty.elapsed += time.delta_secs();

    // Décélération du background (déclenchée par le niveau via StartBgDeceleration)
    if let Some(decel_start) = difficulty.bg_decel_start_elapsed {
        let decel_elapsed = difficulty.elapsed - decel_start;
        let t = (decel_elapsed / difficulty.bg_decel_duration).clamp(0.0, 1.0);
        let bg_speed_at_stop = 150.0 * (1.0 + 8.0 * 3.0);
        let current_speed =
            bg_speed_at_stop + (difficulty.bg_decel_final_speed - bg_speed_at_stop) * t;
        difficulty.bg_speed_override = Some(current_speed);
    }
}
