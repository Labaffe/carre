//! Framework générique pour les ennemis à état machine.
//!
//! Fournit les composants et systèmes réutilisables pour tout ennemi :
//! - État machine : `Entering(u8)` → `Active(usize)` → `Dying` → `Dead`
//! - Phases data-driven via `PhaseDef` (seuil de vie, intervalle, son)
//! - Flash blanc au hit, animation de mort (tremblement, explosions, clignotement)
//! - Projectiles ennemis, mouvement patrol sinusoïdal
//!
//! ## Créer un nouvel ennemi
//! 1. Définir ses constantes et `&'static [PhaseDef]`
//! 2. Spawner une entité avec le composant `Enemy` + composants optionnels
//!    (`PatrolMovement`, `PatternTimer`, animations spécifiques)
//! 3. Écrire les systèmes spécifiques (intro, patterns de tir, mouvement custom)
//! 4. Les systèmes génériques (dégâts, mort, flash, projectiles) fonctionnent automatiquement

use crate::explosion::spawn_explosion;
use crate::missile::{Missile, missile_hits_circle};
use crate::pause::not_paused;
use crate::state::GameState;
use bevy::prelude::*;

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                enemy_hit_flash,
                enemy_phase_logic,
                enemy_dying,
                move_enemy_projectiles,
                cleanup_enemy_projectiles_offscreen,
                missile_enemy_collision,
                patrol_movement,
            )
                .run_if(in_state(GameState::Playing))
                .run_if(not_paused),
        );
    }
}

// ─── État machine ───────────────────────────────────────────────────

/// État générique d'un ennemi.
///
/// - `Entering(u8)` : animation d'intro, le u8 permet plusieurs sous-étapes
///   (ex: 0 = spirale, 1 = flexing pour le boss).
/// - `Active(usize)` : phase de combat, l'index correspond à `enemy.phases[idx]`.
/// - `Dying` : animation de mort (tremblement, explosions).
/// - `Dead` : entité prête à être despawnée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnemyState {
    Entering(u8),
    Active(usize),
    Dying,
    Dead,
}

// ─── Définition de phase ────────────────────────────────────────────

/// Définition statique d'une phase de combat.
/// Tous les champs sont Copy, compatible avec `&'static [PhaseDef]`.
pub struct PhaseDef {
    /// Seuil de vie (fraction 0.0–1.0) pour entrer dans cette phase.
    pub health_threshold_pct: f32,
    /// Intervalle entre les activations de pattern (secondes).
    pub pattern_interval: f32,
    /// Son joué à l'entrée de la phase.
    pub enter_sound: Option<&'static str>,
}

// ─── Composants ─────────────────────────────────────────────────────

/// Composant principal de tout ennemi.
///
/// Contient l'état de jeu (vie, état machine) et la configuration statique
/// (phases, durée de mort, sons). Spawner ce composant suffit pour que
/// les systèmes génériques prennent en charge dégâts, flash et mort.
#[derive(Component)]
pub struct Enemy {
    pub health: i32,
    pub max_health: i32,
    pub state: EnemyState,
    pub radius: f32,
    pub sprite_size: f32,
    pub anim_timer: Timer,
    /// Phases de combat (index 0 = première phase).
    pub phases: &'static [PhaseDef],
    pub death_duration: f32,
    pub death_shake_max: f32,
    pub hit_sound: &'static str,
    pub death_explosion_sound: &'static str,
}

/// Flash blanc au hit.
#[derive(Component)]
pub struct EnemyHitFlash(pub Timer);

/// Position de base pendant l'animation de mort (avant le shake).
#[derive(Component)]
pub struct EnemyDeathAnchor(pub Vec3);

/// Projectile tiré par un ennemi.
#[derive(Component)]
pub struct EnemyProjectile {
    pub velocity: Vec3,
    pub radius: f32,
}

/// Cadence des patterns de tir.
#[derive(Component)]
pub struct PatternTimer(pub Timer);

/// Mouvement patrol : vitesse X constante + sinusoïde Y.
/// Chaque champ est configurable pour varier le comportement par ennemi.
#[derive(Component)]
pub struct PatrolMovement {
    pub dir_x: f32,
    pub sine_time: f32,
    pub initialized: bool,
    /// Si false, le mouvement n'est pas appliqué (utile pour retarder l'activation).
    pub enabled: bool,
    pub speed_x: f32,
    pub sine_amplitude_y: f32,
    pub sine_freq_y: f32,
    pub margin: f32,
}

// ─── Constantes ─────────────────────────────────────────────────────

/// Durée du flash blanc au hit (secondes).
const HIT_FLASH_DURATION: f32 = 0.06;

// ─── Flash blanc au hit ─────────────────────────────────────────────

fn enemy_hit_flash(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Sprite, &mut EnemyHitFlash), With<Enemy>>,
) {
    for (entity, mut sprite, mut flash) in query.iter_mut() {
        flash.0.tick(time.delta());
        if flash.0.finished() {
            sprite.color = Color::WHITE;
            commands.entity(entity).remove::<EnemyHitFlash>();
        } else {
            let t = flash.0.fraction();
            let v = 1.0 + (1.0 - t) * 2.0;
            sprite.color = Color::rgba(v, v, v, 1.0);
        }
    }
}

// ─── Transitions de phase ───────────────────────────────────────────

fn enemy_phase_logic(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut query: Query<(Entity, &mut Enemy, &mut PatternTimer, &Transform)>,
) {
    for (entity, mut enemy, mut pattern_timer, transform) in query.iter_mut() {
        let current_phase = match &enemy.state {
            EnemyState::Active(idx) => *idx,
            _ => continue,
        };

        // Mort → passage en Dying
        if enemy.health <= 0 {
            let duration = enemy.death_duration;
            enemy.state = EnemyState::Dying;
            enemy.anim_timer = Timer::from_seconds(duration, TimerMode::Once);
            commands
                .entity(entity)
                .insert(EnemyDeathAnchor(transform.translation));
            continue;
        }

        let health_pct = enemy.health as f32 / enemy.max_health as f32;

        // Trouver la phase la plus avancée dont le seuil est atteint
        let mut target_phase = current_phase;
        for idx in (0..enemy.phases.len()).rev() {
            if idx > current_phase && health_pct <= enemy.phases[idx].health_threshold_pct {
                target_phase = idx;
                break;
            }
        }

        if target_phase != current_phase {
            let def = &enemy.phases[target_phase];
            enemy.state = EnemyState::Active(target_phase);
            pattern_timer.0 = Timer::from_seconds(def.pattern_interval, TimerMode::Repeating);

            if let Some(sound) = def.enter_sound {
                commands.spawn(AudioBundle {
                    source: asset_server.load(sound),
                    settings: PlaybackSettings::DESPAWN,
                });
            }
        }
    }
}

// ─── Animation de mort ──────────────────────────────────────────────

fn enemy_dying(
    mut commands: Commands,
    time: Res<Time>,
    asset_server: Res<AssetServer>,
    mut query: Query<(
        Entity,
        &mut Enemy,
        &mut Sprite,
        &mut Transform,
        Option<&EnemyDeathAnchor>,
    )>,
) {
    for (entity, mut enemy, mut sprite, mut transform, anchor) in query.iter_mut() {
        if enemy.state != EnemyState::Dying {
            continue;
        }

        enemy.anim_timer.tick(time.delta());
        let progress = enemy.anim_timer.fraction();
        let elapsed = enemy.anim_timer.elapsed_secs();
        let base = anchor.map(|a| a.0).unwrap_or(transform.translation);

        // Clignotement blanc : fréquence augmente (2 Hz → 15 Hz)
        let blink_freq = 2.0 + progress * 13.0;
        let blink = (elapsed * blink_freq * std::f32::consts::TAU).sin() > 0.0;
        if blink {
            let v = 1.0 + (1.0 - progress) * 1.5;
            sprite.color = Color::rgba(v, v, v, 1.0);
        } else {
            sprite.color = Color::WHITE;
        }

        // Tremblement : amplitude quadratique
        let shake = progress * progress * enemy.death_shake_max;
        let shake_x = (fastrand::f32() - 0.5) * 2.0 * shake;
        let shake_y = (fastrand::f32() - 0.5) * 2.0 * shake;
        transform.translation.x = base.x + shake_x;
        transform.translation.y = base.y + shake_y;

        // Explosions : probabilité augmente (5% → 60%)
        let explosion_chance = 0.05 + progress * progress * 0.55;
        if fastrand::f32() < explosion_chance {
            let offset = Vec3::new(
                (fastrand::f32() - 0.5) * 250.0,
                (fastrand::f32() - 0.5) * 250.0,
                1.0,
            );
            spawn_explosion(
                &mut commands,
                &asset_server,
                base + offset,
                Vec2::splat(64.0 + progress * 48.0),
                0,
                Vec3::ZERO,
                Quat::IDENTITY,
            );
            commands.spawn(AudioBundle {
                source: asset_server.load(enemy.death_explosion_sound),
                settings: PlaybackSettings::DESPAWN,
            });
        }

        if enemy.anim_timer.finished() {
            enemy.state = EnemyState::Dead;
            commands.entity(entity).despawn_recursive();
        }
    }
}

// ─── Collision missiles joueur → ennemi ─────────────────────────────

fn missile_enemy_collision(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    missile_q: Query<(Entity, &Transform, &Missile)>,
    mut enemy_q: Query<(Entity, &Transform, &mut Enemy)>,
) {
    for (enemy_entity, enemy_transform, mut enemy) in enemy_q.iter_mut() {
        // Invincible sauf en Active
        match &enemy.state {
            EnemyState::Active(_) => {}
            _ => continue,
        }

        for (missile_entity, missile_transform, missile) in missile_q.iter() {
            let hit = missile_hits_circle(
                missile_transform.translation.truncate(),
                missile_transform.rotation,
                &missile.hitbox,
                enemy_transform.translation.truncate(),
                enemy.radius,
            );

            if hit {
                enemy.health -= 1;
                commands.entity(missile_entity).despawn();

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
        }
    }
}

// ─── Déplacement des projectiles ennemis ────────────────────────────

fn move_enemy_projectiles(
    mut query: Query<(&mut Transform, &EnemyProjectile)>,
    time: Res<Time>,
) {
    for (mut transform, proj) in query.iter_mut() {
        transform.translation += proj.velocity * time.delta_seconds();
    }
}

// ─── Nettoyage des projectiles hors écran ───────────────────────────

fn cleanup_enemy_projectiles_offscreen(
    mut commands: Commands,
    query: Query<(Entity, &Transform), With<EnemyProjectile>>,
) {
    for (entity, transform) in query.iter() {
        let p = transform.translation;
        if p.x.abs() > 1200.0 || p.y.abs() > 900.0 {
            commands.entity(entity).despawn();
        }
    }
}

// ─── Mouvement patrol sinusoïdal ────────────────────────────────────

fn patrol_movement(
    time: Res<Time>,
    mut query: Query<(&Enemy, &mut Transform, &mut PatrolMovement)>,
    windows: Query<&Window>,
) {
    let dt = time.delta_seconds();
    let window = windows.single();

    for (enemy, mut transform, mut patrol) in query.iter_mut() {
        if !patrol.enabled {
            continue;
        }
        match &enemy.state {
            EnemyState::Active(_) => {}
            _ => continue,
        }

        let half_w = window.width() / 2.0 - patrol.margin;
        let half_h = window.height() / 2.0 - patrol.margin;

        // Initialiser sine_time pour démarrer à la position Y actuelle
        if !patrol.initialized {
            let amplitude = half_h * patrol.sine_amplitude_y;
            let ratio = (transform.translation.y / amplitude).clamp(-1.0, 1.0);
            patrol.sine_time = ratio.asin() / patrol.sine_freq_y;
            patrol.initialized = true;
        }

        // X : vitesse constante, flip aux bords
        transform.translation.x += patrol.dir_x * patrol.speed_x * dt;
        if transform.translation.x > half_w {
            transform.translation.x = half_w;
            patrol.dir_x = -1.0;
        } else if transform.translation.x < -half_w {
            transform.translation.x = -half_w;
            patrol.dir_x = 1.0;
        }

        // Y : sinusoïde
        patrol.sine_time += dt;
        let y = (patrol.sine_time * patrol.sine_freq_y).sin() * half_h * patrol.sine_amplitude_y;
        transform.translation.y = y;
    }
}
