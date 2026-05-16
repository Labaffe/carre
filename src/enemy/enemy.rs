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
use bevy::ui::debug::print_ui_layout_tree;


use crate::enemy::enemies::EnemyData;
use crate::enemy::hit_flash::HitFlash;
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
                    // Systèmes réactifs (ordre après la machine à état)
                    projectile_enemy_collision,
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
    pub name: &'static str,
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
    pub fn new(data: EnemyData) -> Self {
        Self {
            radius: data.config.radius,
            sprite_size: data.config.sprite_size,
            name: data.name
        }
    }


    /// `true` si la phase courante permet de prendre des dégâts.
    /// Par défaut, une phase est vulnérable (invulnerable=false).
    pub fn is_vulnerable(&self) -> bool {
        true
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



/// Collision projectiles joueur → ennemi. Inflige `projectile.damage` PV
/// à l'ennemi ciblé si celui-ci est dans une phase vulnérable. Le projectile
/// est toujours détruit au contact, même contre un ennemi invulnérable.
pub fn projectile_enemy_collision(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut score: ResMut<Score>,
    projectile_q: Query<(Entity, &Transform, &Projectile)>,
    mut enemy_q: Query<(Entity, &Transform, &Enemy, &mut Health)>,
) {
    let mut despawned_projectiles = std::collections::HashSet::new();
    for (enemy_entity, enemy_transform, enemy, mut health) in enemy_q.iter_mut() {

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
                    ent.insert(HitFlash(Timer::from_seconds(
                        HIT_FLASH_DURATION,
                        TimerMode::Once,
                    )));
                }
            }

            break; // Ce projectile est consommé, passer au suivant
        }
    }
}



