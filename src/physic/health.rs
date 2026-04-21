//! Composant `Health` unifié, utilisé par toutes les entités qui peuvent
//! prendre des dégâts : joueur, ennemis, astéroïdes.
//!
//! Les systèmes de collision ne manipulent plus des champs `Enemy.health` ou
//! une ressource `PlayerLives` séparée — ils décrémentent directement
//! `Health.current` via `take_damage()`.
//!
//! ## Conventions
//! - `current` et `max` sont en `i32` pour éviter les soucis de comparaison
//!   flottante (un projectile fait `damage: i32` aussi).
//! - Une entité est considérée morte dès que `current <= 0`.
//! - Les transitions `HealthBelow(f)` du framework de behaviors comparent la
//!   fraction `current / max` au seuil `f`.

use bevy::prelude::*;

/// Points de vie d'une entité. Fraîchement spawnée, `current == max`.
#[derive(Component, Debug, Clone, Copy)]
pub struct Health {
    pub current: i32,
    pub max: i32,
}

impl Health {
    /// Crée une santé pleine avec une limite donnée.
    pub fn new(max: i32) -> Self {
        Self { current: max, max }
    }

    /// Inflige `damage` PV. Clamp `current` à 0 minimum.
    pub fn take_damage(&mut self, damage: i32) {
        self.current = (self.current - damage).max(0);
    }

    /// Rend `amount` PV. Clamp `current` à `max` maximum.
    pub fn heal(&mut self, amount: i32) {
        self.current = (self.current + amount).min(self.max);
    }

    /// `true` si `current <= 0`.
    pub fn is_dead(&self) -> bool {
        self.current <= 0
    }

    /// Fraction `current / max` dans [0.0, 1.0].
    pub fn fraction(&self) -> f32 {
        if self.max <= 0 {
            0.0
        } else {
            (self.current as f32 / self.max as f32).clamp(0.0, 1.0)
        }
    }

    /// Réinitialise la santé au max (utilisé lors d'une transition de phase
    /// dans l'ancien framework ennemi multi-phase).
    pub fn reset(&mut self, new_max: i32) {
        self.max = new_max;
        self.current = new_max;
    }
}

/// Événement émis quand une entité reçoit des dégâts. Peut être utilisé
/// pour déclencher des FX (flash, son, etc.) sans que le code de collision
/// ne connaisse ces FX.
#[derive(Event, Debug)]
pub struct DamageEvent {
    pub target: Entity,
    pub amount: i32,
    pub source: Option<Entity>,
}

pub struct HealthPlugin;

impl Plugin for HealthPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<DamageEvent>();
    }
}
