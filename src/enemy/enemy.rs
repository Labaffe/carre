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


use crate::audio::{Sfx, SfxPlayer};
use crate::enemy::enemies::EnemyData;
use crate::enemy::hit_flash::HitFlash;
use crate::fx::explosion::spawn_projectile_death;
use crate::physic::collider::{layers, OverlapEvent};
use crate::physic::health::Health;
use crate::physic::invulnerable::Invulnerable;
use crate::ui::score::Score;
use crate::weapon::projectile::Projectile;

// ═══════════════════════════════════════════════════════════════════════
//  Composant Enemy
// ═══════════════════════════════════════════════════════════════════════

/// Composant marker pour tout ennemi. Le nom sert au debug uniquement —
/// la hitbox passe maintenant par le composant `Hitbox` (collider unifié)
/// et la taille du sprite par `Sprite.custom_size`.
#[derive(Component)]
pub struct Enemy {
    pub name: &'static str,
}

impl Enemy {
    pub fn new(data: EnemyData) -> Self {
        Self { name: data.name }
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
#[derive(Message)]
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



/// Réactif sur `OverlapEvent` : pour chaque overlap projectile joueur ↔
/// (ennemi | astéroïde), inflige les dégâts et despawn le projectile.
/// Unifie l'ancien `projectile_enemy_collision` et le dead `projectile_asteroid_collision`.
///
/// Comportement :
/// - Projectile toujours despawn (même si cible invulnérable). Anim de mort
///   du projectile via `spawn_projectile_death` si `death_folder` configuré.
/// - Cible prend `projectile.damage` PV si pas `Invulnerable` ET (si Enemy)
///   en phase vulnérable.
/// - HitFlash + Sfx::EnemyHit + score+1 sur dégât effectif.
/// - Mort (HP=0) gérée par les systèmes existants (`detect_death`, `asteroid_death_fx`).
pub fn projectile_damage_on_overlap(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut events: MessageReader<OverlapEvent>,
    projectile_q: Query<(&Transform, &Projectile)>,
    mut target_q: Query<(&mut Health, Option<&Invulnerable>)>,
    mut score: ResMut<Score>,
    mut sfx: SfxPlayer,
) {
    let mut despawned_projectiles = std::collections::HashSet::new();

    for ev in events.read() {
        let Some((proj_e, target_e)) = ev.pick(layers::PLAYER_PROJECTILE) else { continue };
        // Filtre cible : ENEMY ou ASTEROID uniquement
        let target_layer = if ev.a == target_e { ev.a_layer } else { ev.b_layer };
        if target_layer & (layers::ENEMY | layers::ASTEROID) == 0 {
            continue;
        }
        // Un projectile ne touche qu'une cible (premier event consommé)
        if despawned_projectiles.contains(&proj_e) {
            continue;
        }

        let Ok((proj_tf, projectile)) = projectile_q.get(proj_e) else { continue };
        let Ok((mut health, invulnerable)) = target_q.get_mut(target_e) else { continue };

        // Despawn projectile + anim de mort (même si cible invulnérable)
        spawn_projectile_death(
            &mut commands,
            &asset_server,
            proj_tf.translation,
            projectile.death_folder,
        );
        if let Ok(mut e) = commands.get_entity(proj_e) {
            e.try_despawn();
        }
        despawned_projectiles.insert(proj_e);

        // Dégâts si cible vulnérable (= pas de marker Invulnerable).
        if invulnerable.is_none() {
            health.take_damage(projectile.damage);
            score.add(1);
            if let Ok(mut ent) = commands.get_entity(target_e) {
                ent.insert(HitFlash(Timer::from_seconds(
                    HIT_FLASH_DURATION,
                    TimerMode::Once,
                )));
            }
            sfx.play(Sfx::EnemyHit);
        }
    }
}



