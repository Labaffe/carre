//! Framework générique pour tous les ennemis.
//!
//! **Composant unique** : `Enemy`. Il contient à la fois la config statique
//! (radius, sprite, sons) et la machine à état data-driven (`EnemyDefinition`
//! + `current_phase` + `phase_timer`).
//!
//! ## Architecture
//! - Une entité ennemie a **toujours** un composant `Enemy` et un composant
//!   `Health`.
//! - Le cerveau est dans `Enemy.definition` : une liste de `Phase`s avec des
//!   `Behavior`s et des `Transition`s (cf. `enemy/system.rs`).
//! - Deux systèmes génériques tournent chaque frame :
//!   - `phase_transition_system` : tick du timer + évaluation des transitions
//!     (+ on_enter de la phase cible si transition).
//!   - `behavior_execution_system` : exécute le `behavior` de la phase en cours.
//! - Les systèmes spécifiques (flash hit, collision projectile, etc.) sont
//!   dans ce module.
//!
//! ## Créer un nouvel ennemi
//! 1. Construire une `EnemyDefinition` avec ses phases et behaviors
//! 2. Spawner une entité avec `Enemy::new(config, definition)` + `Health` +
//!    `SpriteBundle` + marqueurs custom éventuels
//! 3. Les systèmes génériques prennent en charge dégâts, flash, transitions

use bevy::prelude::*;

use crate::enemy::system::{
    behavior_execution_system, phase_transition_system, EnemyDefinition, Phase, PhaseId,
};
use crate::game_manager::state::GameState;
use crate::item::item::{DropEvent, DropTable};
use crate::menu::pause::not_paused;
use crate::physic::health::Health;
use crate::ui::score::Score;
use crate::weapon::projectile::{projectile_hits_circle, Projectile, Team};

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<EnemyDeathEvent>()
            .add_systems(
                Update,
                (
                    // Framework phases+behaviors (exclusif, séquentiel)
                    phase_transition_system,
                    behavior_execution_system,
                    // Systèmes réactifs (ordre après la machine à état)
                    enemy_hit_flash,
                    projectile_enemy_collision,
                    enemy_death_detection,
                )
                    .chain()
                    .run_if(in_state(GameState::Playing))
                    .run_if(not_paused),
            );
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Composant Enemy
// ═══════════════════════════════════════════════════════════════════════

/// Composant principal de tout ennemi. Porte la config statique et la
/// machine à état data-driven.
#[derive(Component)]
pub struct Enemy {
    // ─── Config statique (immutable après spawn) ───
    pub radius: f32,
    pub sprite_size: f32,
    pub hit_sound: &'static str,
    pub death_explosion_sound: &'static str,
    /// Couleur du flash au hit. None = blanc pur (par défaut).
    pub hit_flash_color: Option<Color>,

    // ─── Machine à état ───
    pub definition: EnemyDefinition,
    pub current_phase: PhaseId,
    pub phase_timer: Timer,
}

/// Config statique d'un ennemi (radius, sons, couleurs). Utilisé pour
/// construire un `Enemy` via `Enemy::new(config, definition)`.
pub struct EnemyConfig {
    pub radius: f32,
    pub sprite_size: f32,
    pub hit_sound: &'static str,
    pub death_explosion_sound: &'static str,
    pub hit_flash_color: Option<Color>,
}

impl Enemy {
    pub fn new(config: EnemyConfig, definition: EnemyDefinition) -> Self {
        let initial_phase = definition.initial_phase.clone();
        Self {
            radius: config.radius,
            sprite_size: config.sprite_size,
            hit_sound: config.hit_sound,
            death_explosion_sound: config.death_explosion_sound,
            hit_flash_color: config.hit_flash_color,
            definition,
            current_phase: initial_phase,
            phase_timer: Timer::from_seconds(0.0, TimerMode::Once),
        }
    }

    /// Phase courante (donne accès à son `invulnerable`, ses transitions, etc.).
    pub fn current_phase_def(&self) -> Option<&Phase> {
        self.definition.get_phase(&self.current_phase)
    }

    /// `true` si la phase courante permet de prendre des dégâts.
    /// Par défaut, une phase est vulnérable (invulnerable=false).
    pub fn is_vulnerable(&self) -> bool {
        self.current_phase_def()
            .map(|p| !p.invulnerable)
            .unwrap_or(false)
    }

    /// `true` si la phase courante est `"dead"` (l'entité va être despawnée
    /// par `DespawnSelf` au prochain frame).
    pub fn is_dead_phase(&self) -> bool {
        self.current_phase == PhaseId("dead")
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Composants auxiliaires
// ═══════════════════════════════════════════════════════════════════════

/// Flash blanc temporaire appliqué quand l'ennemi prend un hit.
#[derive(Component)]
pub struct EnemyHitFlash(pub Timer);

/// Position de référence pour une animation de shake (utilisée par
/// les behaviors `ShakeAround` / `DyingFx` pour reprendre la position
/// initiale d'une phase).
#[derive(Component)]
pub struct EnemyDeathAnchor(pub Vec3);

/// Événement émis quand un ennemi atteint PV=0 pour la première fois.
/// Permet aux systèmes spécifiques (drop d'items, etc.) de réagir sans
/// être couplés au moteur de phases.
#[derive(Event)]
pub struct EnemyDeathEvent {
    pub entity: Entity,
    pub position: Vec3,
}

// ═══════════════════════════════════════════════════════════════════════
//  Constantes
// ═══════════════════════════════════════════════════════════════════════

/// Durée du flash blanc au hit (secondes).
const HIT_FLASH_DURATION: f32 = 0.06;

// ═══════════════════════════════════════════════════════════════════════
//  Systèmes
// ═══════════════════════════════════════════════════════════════════════

/// Applique un flash blanc quand l'entité a un `EnemyHitFlash` et le retire
/// quand le timer expire.
fn enemy_hit_flash(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Sprite, &mut EnemyHitFlash, &Enemy)>,
) {
    for (entity, mut sprite, mut flash, enemy) in query.iter_mut() {
        flash.0.tick(time.delta());
        if flash.0.finished() {
            sprite.color = Color::WHITE;
            commands.entity(entity).remove::<EnemyHitFlash>();
        } else {
            sprite.color = enemy
                .hit_flash_color
                .unwrap_or(Color::rgba(100.0, 100.0, 100.0, 1.0));
        }
    }
}

/// Collision projectiles joueur → ennemi. Inflige `projectile.damage` PV
/// à l'ennemi ciblé si celui-ci est dans une phase vulnérable. Le projectile
/// est toujours détruit au contact, même contre un ennemi invulnérable.
fn projectile_enemy_collision(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut score: ResMut<Score>,
    projectile_q: Query<(Entity, &Transform, &Projectile)>,
    mut enemy_q: Query<(Entity, &Transform, &Enemy, &mut Health)>,
) {
    let mut despawned_projectiles = std::collections::HashSet::new();

    for (enemy_entity, enemy_transform, enemy, mut health) in enemy_q.iter_mut() {
        if enemy.is_dead_phase() {
            continue;
        }

        for (projectile_entity, projectile_transform, projectile) in projectile_q.iter() {
            if projectile.team != Team::Player {
                continue;
            }
            if despawned_projectiles.contains(&projectile_entity) {
                continue;
            }

            let hit = projectile_hits_circle(
                projectile_transform.translation.truncate(),
                projectile_transform.rotation,
                &projectile.hitbox,
                enemy_transform.translation.truncate(),
                enemy.radius,
            );
            if !hit {
                continue;
            }

            // Le projectile est détruit même contre un ennemi invulnérable.
            if let Some(mut e) = commands.get_entity(projectile_entity) {
                e.despawn();
            }
            despawned_projectiles.insert(projectile_entity);

            if enemy.is_vulnerable() {
                health.take_damage(projectile.damage);
                score.add(1);

                if let Some(mut ent) = commands.get_entity(enemy_entity) {
                    ent.insert(EnemyHitFlash(Timer::from_seconds(
                        HIT_FLASH_DURATION,
                        TimerMode::Once,
                    )));
                }
                commands.spawn(AudioBundle {
                    source: asset_server.load(enemy.hit_sound),
                    settings: PlaybackSettings::DESPAWN,
                });
            }

            break; // Ce projectile est consommé, passer au suivant
        }
    }
}

/// Détecte les passages en PV=0 et émet un `EnemyDeathEvent` + drop table.
/// Le nettoyage visuel/sonore est pris en charge par les behaviors de la
/// phase `"dying"` de chaque définition.
fn enemy_death_detection(
    mut enemy_q: Query<(Entity, &Transform, &Enemy, &Health, Option<&DropTable>), Changed<Health>>,
    mut drop_events: EventWriter<DropEvent>,
    mut death_events: EventWriter<EnemyDeathEvent>,
) {
    for (entity, transform, enemy, health, drop_table) in enemy_q.iter_mut() {
        if !health.is_dead() {
            continue;
        }
        // Ne pas redéclencher si l'ennemi est déjà en phase dying/dead
        if enemy.current_phase == PhaseId("dying") || enemy.current_phase == PhaseId("dead") {
            continue;
        }
        if let Some(table) = drop_table {
            drop_events.send(DropEvent {
                position: transform.translation,
                table: table.drops,
            });
        }
        death_events.send(EnemyDeathEvent {
            entity,
            position: transform.translation,
        });
    }
}

