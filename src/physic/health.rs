//! Composant `Health` + pipeline de dégâts unifié via événements.
//!
//! ## Pipeline
//!
//! ```text
//! Émetteurs ──→ DamageEvent ──→ apply_damage ──→ HitEvent ──→ FX reactive systems
//! (projectile,    (target,        (un seul          (target,         (HitFlash,
//!  player coll,    amount,         endroit où        amount_dealt,    Sfx, Score,
//!  bomb...)        source)         on check          source,          player Invincible,
//!                                  Invulnerable      target_layer)    etc.)
//!                                  + Invincible
//!                                  + take_damage)
//! ```
//!
//! Avantage : les règles métier de l'application (skip si invulnerable, etc.)
//! vivent à UN SEUL endroit (`apply_damage`). Les émetteurs disent juste
//! "faire X dégâts à Y". Les FX listent indépendamment "j'ai pris un hit".

use bevy::prelude::*;

use crate::physic::collider::CollisionLayer;
use crate::physic::invulnerable::{DebugInvulnerable, Invulnerable};
use crate::player::player::Invincible;

/// Points de vie d'une entité. Fraîchement spawnée, `current == max`.
#[derive(Component, Debug, Clone, Copy)]
pub struct Health {
    pub current: i32,
    pub max: i32,
}

impl Health {
    pub fn new(max: i32) -> Self {
        Self { current: max, max }
    }
    pub fn take_damage(&mut self, damage: i32) {
        self.current = (self.current - damage).max(0);
    }
    pub fn heal(&mut self, amount: i32) {
        self.current = (self.current + amount).min(self.max);
    }
    pub fn is_dead(&self) -> bool {
        self.current <= 0
    }
    pub fn fraction(&self) -> f32 {
        if self.max <= 0 {
            0.0
        } else {
            (self.current as f32 / self.max as f32).clamp(0.0, 1.0)
        }
    }
    pub fn reset(&mut self, new_max: i32) {
        self.max = new_max;
        self.current = new_max;
    }
}

// ─── Events ──────────────────────────────────────────────────────────

/// Demande d'infliction de dégâts. Émis par les systèmes de collision ou
/// d'effet (projectile, bombe, contact joueur). Consommé par `apply_damage`.
#[derive(Message, Debug, Clone, Copy)]
pub struct DamageEvent {
    pub target: Entity,
    pub amount: i32,
    /// Source du dégât (projectile, ennemi, …). `None` pour les broadcasts
    /// non-spatiaux comme la bombe.
    pub source: Option<Entity>,
}

/// Notification que des dégâts ont été appliqués (post-filtrage Invulnerable).
/// Émis par `apply_damage` via `commands.trigger(...)`. Consommé par des
/// **observers globaux** : FX (flash, son, score) dans `enemy.rs` + logique
/// player (Invincible/GameOver, son hurt) dans `collision.rs`.
#[derive(Event, Debug, Clone, Copy)]
pub struct HitEvent {
    pub target: Entity,
    /// Layer de la cible (utilisé pour dispatcher les FX selon le type
    /// d'entité touchée).
    pub target_layer: u32,
    pub amount_dealt: i32,
    pub source: Option<Entity>,
}

// ─── Système central ─────────────────────────────────────────────────

/// Lit `DamageEvent`, filtre Invulnerable/Invincible, applique à `Health`,
/// trigger `HitEvent` si le dégât a effectivement été infligé.
pub fn apply_damage(
    mut commands: Commands,
    mut damage_events: MessageReader<DamageEvent>,
    mut q: Query<(
        &mut Health,
        &CollisionLayer,
        Option<&Invulnerable>,
        Option<&DebugInvulnerable>,
        Option<&Invincible>,
    )>,
) {
    for ev in damage_events.read() {
        let Ok((mut health, layer, invulnerable, debug_invulnerable, invincible)) =
            q.get_mut(ev.target)
        else {
            continue;
        };
        if invulnerable.is_some() || debug_invulnerable.is_some() || invincible.is_some() {
            continue;
        }
        let before = health.current;
        health.take_damage(ev.amount);
        let dealt = before - health.current;
        if dealt > 0 {
            commands.trigger(HitEvent {
                target: ev.target,
                target_layer: layer.0,
                amount_dealt: dealt,
                source: ev.source,
            });
        }
    }
}

// ─── Plugin ──────────────────────────────────────────────────────────

pub struct HealthPlugin;

impl Plugin for HealthPlugin {
    fn build(&self, app: &mut App) {
        // `HitEvent` n'est PAS enregistré via `add_message` : il est dispatché
        // exclusivement via `commands.trigger` vers les observers (cf. la
        // chaîne `add_observer(...)` dans `EnemyPlugin` et `CollisionPlugin`).
        app.add_message::<DamageEvent>()
            .add_systems(
                Update,
                apply_damage.run_if(in_state(crate::game_manager::state::GameState::Playing)),
            );
    }
}
