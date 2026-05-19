//! Niveau "Chaos" : spawn aléatoire d'ennemis depuis une palette configurable.
//!
//! Le `ChaosConfig` (Resource) pilote tout. Tant qu'il est inséré, le système
//! [`chaos_spawner_system`] tick un timer interne et pousse une `SpawnRequest`
//! aléatoire dans `difficulty.spawn_requests` à chaque cycle. Les systèmes
//! de spawn de chaque ennemi consomment ces requêtes comme d'habitude.
//!
//! ## Paramétrage
//! ```ignore
//! ChaosConfig::new()
//!     .with_interval(2.0)              // 1 spawn toutes les 2s
//!     .exclude("kamikaze")             // retire un ennemi de la palette
//!     .with_position(SpawnPosition::Top) // d'où ils apparaissent
//! ```
//!
//! Le **boss est exclu par défaut** — la palette initiale est
//! `["asteroid", "green_ufo", "mine", "kamikaze"]`.

use crate::enemy::enemy_register::EnemyRegister;
use crate::game_manager::difficulty::{Difficulty, SpawnPosition};
use bevy::prelude::*;

/// Configuration du spawner du niveau Chaos.
#[derive(Resource, Clone)]
pub struct ChaosConfig {
    /// Palette d'ennemis qui peuvent spawner. Modifiable via `include`/`exclude`.
    pub enemies: Vec<&'static str>,
    /// Intervalle entre 2 spawns (secondes).
    pub spawn_interval: f32,
    /// Position de spawn (par défaut `Top` — les ennemis arrivent par le haut).
    pub spawn_position: SpawnPosition,
    /// Timer interne (driven par le système, ne pas modifier à la main).
    pub timer: Timer,
}

impl Default for ChaosConfig {
    fn default() -> Self {
        let interval = 1.5;
        Self {
            enemies: vec!["asteroid", "green_ufo", "mine", "kamikaze"],
            spawn_interval: interval,
            spawn_position: SpawnPosition::Top,
            timer: Timer::from_seconds(interval, TimerMode::Repeating),
        }
    }
}

impl ChaosConfig {
    pub fn new() -> Self {
        Self::default()
    }

    /// Construit la palette à partir de tous les ennemis enregistrés dans
    /// `EnemyRegister`, moins ceux listés dans `excluded`. Cas d'usage
    /// typique : `from_register(&register, &["boss"])` → tous les ennemis
    /// sauf le boss, peu importe ceux ajoutés au register plus tard.
    pub fn from_register(register: &EnemyRegister, excluded: &[&'static str]) -> Self {
        let enemies: Vec<&'static str> = register
            .0
            .iter()
            .map(|b| b.name())
            .filter(|n| !excluded.contains(n))
            .collect();
        let mut config = Self::default();
        config.enemies = enemies;
        config
    }

    /// Change l'intervalle entre spawns (resette le timer interne).
    pub fn with_interval(mut self, interval: f32) -> Self {
        self.spawn_interval = interval;
        self.timer = Timer::from_seconds(interval, TimerMode::Repeating);
        self
    }

    /// Change la position de spawn de tous les ennemis.
    pub fn with_position(mut self, pos: SpawnPosition) -> Self {
        self.spawn_position = pos;
        self
    }

    /// Retire un ennemi de la palette (si présent).
    pub fn exclude(mut self, enemy: &'static str) -> Self {
        self.enemies.retain(|e| *e != enemy);
        self
    }

    /// Ajoute un ennemi à la palette (si pas déjà présent). Le boss n'est pas
    /// dans le défaut mais peut être réintroduit via cette méthode.
    pub fn include(mut self, enemy: &'static str) -> Self {
        if !self.enemies.contains(&enemy) {
            self.enemies.push(enemy);
        }
        self
    }
}

/// Tick le timer du chaos et, à chaque expiration, pousse une `SpawnRequest`
/// d'un ennemi aléatoire de la palette dans `difficulty.spawn_requests`.
pub fn chaos_spawner_system(
    time: Res<Time>,
    mut chaos: ResMut<ChaosConfig>,
    mut difficulty: ResMut<Difficulty>,
) {
    if chaos.enemies.is_empty() {
        return;
    }
    chaos.timer.tick(time.delta());
    if chaos.timer.just_finished() {
        let idx = fastrand::usize(0..chaos.enemies.len());
        let enemy = chaos.enemies[idx];
        let pos = chaos.spawn_position;
        difficulty.spawn_requests.push((enemy, 1, pos));
    }
}
