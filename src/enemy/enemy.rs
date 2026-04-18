//! Framework générique pour les ennemis à état machine.
//!
//! Fournit les composants et systèmes réutilisables pour tout ennemi :
//! - État machine : `Entering` → `Flexing` → `Active(usize)` → `Dying` → `Dead`
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

use crate::enemy::system::EnemyBehavior;
use crate::fx::explosion::spawn_explosion;
use crate::game_manager::state::GameState;
use crate::item::item::{DropEvent, DropTable};
use crate::menu::pause::not_paused;
use crate::physic::health::Health;
use crate::ui::score::Score;
use crate::weapon::projectile::{projectile_hits_circle, Projectile, Team};
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
                projectile_enemy_collision,
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
/// ```text
///  Entering ──→ Flexing ──→ Idle ──→ Active(0) ──→ Active(1) ──→ … ──→ Dying ──→ Dead
/// ```
///
/// Chaque état est optionnel sauf `Active` et `Dead`.
///
/// - `Entering` : animation d'arrivée (ex: spirale du boss).
/// - `Flexing` : animation post-arrivée (ex: pose du boss).
/// - `Idle` : attente avant le combat. L'ennemi joue son animation idle,
///    ne bouge pas, ne tire pas, est invulnérable. Utile pour laisser la
///    musique démarrer avant d'engager le combat.
/// - `Active(usize)` : phase de combat, l'index correspond à `enemy.phases[idx]`.
///    Les patterns ne se déclenchent que dans cet état.
/// - `Dying` : animation de mort (tremblement, explosions, clignotement).
/// - `Dead` : entité despawnée.
///
/// Un ennemi simple peut spawner directement en `Active(0)`.
/// Un boss utilise `Entering` → `Flexing` → `Idle` → `Active(0)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnemyState {
    /// Animation d'arrivée (optionnelle).
    Entering,
    /// Animation post-arrivée (optionnelle, ex: flexing du boss).
    Flexing,
    /// Attente avant le combat (optionnelle). Idle, invulnérable, immobile.
    Idle,
    /// Phase de combat. Seul état où les patterns se déclenchent.
    Active(usize),
    /// Transition entre deux phases (shake + flash, pas d'explosions).
    /// L'index est la phase **suivante** vers laquelle on transite.
    Transitioning(usize),
    /// Animation de mort en cours, invincible.
    Dying,
    /// Mort, sera despawné.
    Dead,
}

// ─── Définition de phase ────────────────────────────────────────────

/// Définition statique d'un pattern de combat.
pub struct PatternDef {
    /// Nom du pattern (ex: "charge", "patrol").
    pub name: &'static str,
    /// Durée avant l'activation de ce pattern (secondes).
    pub duration: f32,
}

/// Définition statique d'une phase de combat.
///
/// Chaque phase a son propre pool de points de vie.
/// Quand les PV tombent à 0, l'ennemi passe à la phase suivante.
/// À la dernière phase, 0 PV = mort.
pub struct PhaseDef {
    /// Points de vie de cette phase.
    pub health: i32,
    /// Son joué à l'entrée de la phase.
    pub enter_sound: Option<&'static str>,
    /// Patterns de combat, chacun avec son propre timing.
    /// Le pattern executor cycle à travers cette liste.
    pub patterns: &'static [PatternDef],
    /// Si `true`, une animation de transition (shake + flash) est jouée
    /// avant de passer à la phase suivante quand les PV tombent à 0.
    /// L'animation est gérée par le module spécifique de l'ennemi.
    pub has_transition: bool,
}

// ─── Composants ─────────────────────────────────────────────────────

/// Composant principal de tout ennemi.
///
/// Contient l'état de jeu (vie, état machine) et la configuration statique
/// (phases, durée de mort, sons). Spawner ce composant suffit pour que
/// les systèmes génériques prennent en charge dégâts, flash et mort.
#[derive(Component)]
pub struct Enemy {
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
    /// Couleur du flash au hit. None = blanc pur (par défaut).
    pub hit_flash_color: Option<Color>,
}

/// Flash blanc au hit.
#[derive(Component)]
pub struct EnemyHitFlash(pub Timer);

/// Position de base pendant l'animation de mort (avant le shake).
#[derive(Component)]
pub struct EnemyDeathAnchor(pub Vec3);

/// Cadence des patterns de tir.
#[derive(Component)]
pub struct PatternTimer(pub Timer);

/// Index du pattern actuel dans la liste de patterns de la phase.
/// Composant générique utilisé par tous les ennemis.
#[derive(Component)]
pub struct PatternIndex(pub usize);

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
    mut query: Query<(Entity, &mut Sprite, &mut EnemyHitFlash, &Enemy)>,
) {
    for (entity, mut sprite, mut flash, enemy) in query.iter_mut() {
        flash.0.tick(time.delta());
        if flash.0.finished() {
            sprite.color = Color::WHITE;
            commands.entity(entity).remove::<EnemyHitFlash>();
        } else {
            sprite.color = enemy.hit_flash_color
                .unwrap_or(Color::rgba(100.0, 100.0, 100.0, 1.0));
        }
    }
}

// ─── Transitions de phase ───────────────────────────────────────────

/// Gestion des transitions de phase par seuil de PV pour les ennemis "old-style"
/// (machine `EnemyState` classique). Les entités pilotées par `EnemyBehavior`
/// sont ignorées — elles gèrent leurs transitions via `HealthBelow`.
fn enemy_phase_logic(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut query: Query<
        (
            Entity,
            &mut Enemy,
            &mut Health,
            &mut PatternTimer,
            &Transform,
            Option<&DropTable>,
        ),
        Without<EnemyBehavior>,
    >,
    mut drop_events: EventWriter<DropEvent>,
) {
    for (entity, mut enemy, mut health, mut pattern_timer, transform, drop_table) in
        query.iter_mut()
    {
        let current_phase = match &enemy.state {
            EnemyState::Active(idx) => *idx,
            _ => continue,
        };

        if health.current > 0 {
            continue;
        }

        // PV à 0 → phase suivante (ou transition) ou mort
        let current_def = &enemy.phases[current_phase];
        let next_phase = current_phase + 1;
        if next_phase < enemy.phases.len() {
            if current_def.has_transition {
                // Transition animée avant la phase suivante
                enemy.state = EnemyState::Transitioning(next_phase);
                health.current = 1; // invulnérable pendant la transition, évite de re-trigger
                // Le module spécifique de l'ennemi gère l'animation et le passage en Active
            } else {
                // Transition directe vers la phase suivante
                let def = &enemy.phases[next_phase];
                enemy.state = EnemyState::Active(next_phase);
                health.reset(def.health);
                let first_duration = def.patterns.first().map(|p| p.duration).unwrap_or(1.0);
                pattern_timer.0 = Timer::from_seconds(first_duration, TimerMode::Once);

                if let Some(sound) = def.enter_sound {
                    commands.spawn(AudioBundle {
                        source: asset_server.load(sound),
                        settings: PlaybackSettings::DESPAWN,
                    });
                }
            }
        } else {
            // Dernière phase → mort
            let duration = enemy.death_duration;
            enemy.state = EnemyState::Dying;
            enemy.anim_timer = Timer::from_seconds(duration, TimerMode::Once);
            commands
                .entity(entity)
                .insert(EnemyDeathAnchor(transform.translation));

            if let Some(table) = drop_table {
                drop_events.send(DropEvent {
                    position: transform.translation,
                    table: table.drops,
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
    mut query: Query<
        (
            Entity,
            &mut Enemy,
            &mut Sprite,
            &mut Transform,
            Option<&EnemyDeathAnchor>,
        ),
        Without<EnemyBehavior>,
    >,
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
            if let Some(e) = commands.get_entity(entity) {
                e.despawn_recursive();
            }
        }
    }
}

// ─── Collision projectiles joueur → ennemi ──────────────────────────

fn projectile_enemy_collision(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut score: ResMut<Score>,
    projectile_q: Query<(Entity, &Transform, &Projectile)>,
    mut enemy_q: Query<(Entity, &Transform, &Enemy, &mut Health)>,
) {
    let mut despawned_projectiles = std::collections::HashSet::new();

    for (enemy_entity, enemy_transform, enemy, mut health) in enemy_q.iter_mut() {
        // Ignorer les ennemis morts
        if matches!(enemy.state, EnemyState::Dying | EnemyState::Dead) {
            continue;
        }

        // "Vulnérable" au sens du flash/son : uniquement en Active pour les
        // ennemis old-style ; les new-style (EnemyBehavior) sont toujours
        // vulnérables en Active(0) par convention.
        let is_vulnerable = matches!(enemy.state, EnemyState::Active(_));

        for (projectile_entity, projectile_transform, projectile) in projectile_q.iter() {
            // Seuls les projectiles du joueur blessent les ennemis
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
            if hit {
                // Le projectile est toujours détruit au contact
                if let Some(mut e) = commands.get_entity(projectile_entity) {
                    e.despawn();
                }
                despawned_projectiles.insert(projectile_entity);

                if is_vulnerable {
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
}

// ─── (Le mouvement et le cleanup offscreen des projectiles sont gérés
//      par `ProjectilePlugin` dans weapon/projectile.rs)

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
