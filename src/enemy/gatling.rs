//! Gatling — tourelles attachées au Mothership ou standalone.
//!
//! Machine à état : Entering (animation sprite) → Active(0) → Dying → Dead
//!
//! Patterns :
//!   - `aim_and_shoot` : suivi continu du joueur + tir unique en fin de pattern
//!   - `full_auto` : balayage ping-pong avec tir à cadence croissante
//!   - `idle` : pause (aucun mouvement)
//!
//! Les types partagés (MothershipConfig, EntryEdge, composants, etc.)
//! sont dans `mothership.rs`. Ce module ne contient que le code Gatling.

use crate::enemies::GATLING;
use crate::enemy::{Enemy, EnemyState, PatternIndex, PatternTimer};
use crate::weapon::projectile::{spawn_projectile, ProjectileSpawn, ProjectileSprite, Team};
use crate::weapon::weapon::HitboxShape;
use crate::item::DropTable;
use crate::mothership::{
    EntryEdge, GATLING_ANIM_INTERVAL, GATLING_SPRITE_SIZE, GatlingAimBias, GatlingFrames,
    GatlingLaser, GatlingMarker, GatlingPatternOverride, GatlingStyleComp, MOTHERSHIP_DROP_TABLE,
    MOTHERSHIP_ENTERING_DURATION, MothershipLink, MothershipSpawnQueue,
};
use crate::pause::not_paused;
use crate::player::Player;
use crate::state::GameState;
use bevy::prelude::*;

pub struct GatlingPlugin;

impl Plugin for GatlingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MothershipSpawnQueue>()
            .add_systems(Startup, crate::mothership::preload_gatling_frames)
            .add_systems(
                Update,
                (
                    // Mothership systems (from mothership.rs)
                    crate::mothership::spawn_mothership_oneshot,
                    crate::mothership::mothership_entering,
                    crate::mothership::mothership_drift,
                    crate::mothership::mothership_sync_positions,
                    crate::mothership::mothership_death_detection,
                    crate::mothership::mothership_dying,
                    // Gatling systems (local)
                    spawn_gatlings_oneshot,
                    gatling_standalone_entering,
                    gatling_entering_animate,
                    gatling_pattern_executor,
                    gatling_shoot_update,
                    gatling_full_auto_update,
                    gatling_laser_update,
                )
                    .run_if(in_state(GameState::Playing))
                    .run_if(not_paused),
            );
    }
}

// ─── Constantes Gatling ────────────────────────────────────────────

/// Angle de rotation max vers le joueur (degrés, demi-cône de visée).
const GATLING_AIM_MAX_ANGLE: f32 = 45.0;
/// Vitesse de rotation vers le joueur (degrés/seconde).
const GATLING_AIM_SPEED: f32 = 120.0;
/// Distance max (pixels) à laquelle la tourelle aim_and_shoot peut verrouiller le joueur.
/// Au-delà, elle balaie sa zone à la recherche du joueur.
const GATLING_AIM_DETECTION_RANGE: f32 = 450.0;
/// Pulsation du balayage en mode recherche (radians/seconde).
const GATLING_SWEEP_FREQUENCY: f32 = 1.4;
// Vitesse et rayon de projectile par défaut — définis dans GatlingStyleComp::default().
/// Intervalle entre deux frames de l'animation de tir (secondes).
const GATLING_SHOOT_ANIM_INTERVAL: f32 = 0.1;

// ─── Full Auto ─────────────────────────────────────────────────────

/// Vitesse de balayage initiale (degrés/seconde).
const FULL_AUTO_SWEEP_SPEED_START: f32 = 30.0;
/// Vitesse de balayage maximale (degrés/seconde).
const FULL_AUTO_SWEEP_SPEED_MAX: f32 = 180.0;
/// Intervalle de tir initial (secondes entre chaque tir).
const FULL_AUTO_FIRE_INTERVAL_START: f32 = 0.8;
/// Intervalle de tir minimal (cadence max).
const FULL_AUTO_FIRE_INTERVAL_MIN: f32 = 0.15;
/// Courbe d'accélération (>1 = montée lente, <1 = montée rapide).
const FULL_AUTO_RAMP_FACTOR: f32 = 3.5;
/// Intervalle entre deux frames de l'animation de tir en full auto (secondes).
const FULL_AUTO_SHOOT_ANIM_INTERVAL: f32 = 0.04;

/// Distance parcourue pendant l'Entering standalone (pixels).
const GATLING_ENTERING_DISTANCE: f32 = 900.0;

// ─── Composants Gatling-spécifiques ────────────────────────────────

/// Animation de la Gatling pendant l'Entering.
#[derive(Component)]
pub(crate) struct GatlingEnteringAnim {
    pub(crate) timer: Timer,
    pub(crate) current_frame: usize,
}

/// Position Y de départ pour un Gatling standalone (sans Mothership).
#[derive(Component)]
struct GatlingStartY(f32);

/// Composant actif pendant le pattern "aim_and_shoot".
#[derive(Component)]
struct GatlingShoot {
    target_angle: f32,
    current_angle: f32,
    elapsed: f32,
    duration: f32,
    anim_timer: Timer,
    current_frame: usize,
    fired: bool,
    anim_started: bool,
    /// true si le joueur est actuellement verrouillé (dans la portée + cône).
    player_locked: bool,
}

/// Composant actif pendant le pattern "full_auto".
#[derive(Component)]
struct GatlingFullAuto {
    current_angle: f32,
    sweep_dir: f32,
    startup_delay: f32,
    elapsed: f32,
    duration: f32,
    fire_timer: Timer,
    anim_frame: Option<usize>,
    anim_timer: Timer,
}

/// Stocke le `EntryEdge` du Mothership parent pour calculer l'angle de base.
#[derive(Component)]
pub(crate) struct GatlingBaseEdge(pub(crate) EntryEdge);

// ─── Spawn Gatling standalone (via spawn_requests "gatling") ───────

fn spawn_gatlings_oneshot(
    mut commands: Commands,
    mut difficulty: ResMut<crate::difficulty::Difficulty>,
    frames: Res<GatlingFrames>,
    windows: Query<&Window>,
) {
    let Some(pos_idx) = difficulty
        .spawn_requests
        .iter()
        .position(|(name, _, _)| *name == "gatling")
    else {
        return;
    };
    let (_name, count, spawn_pos) = difficulty.spawn_requests.remove(pos_idx);

    let window = windows.single();
    for _ in 0..count {
        let pos = spawn_pos.resolve(window, 60.0);
        let phase = &GATLING.phases[0];
        let first_frame = frames.0.first().cloned().unwrap_or_default();

        commands.spawn((
            SpriteBundle {
                texture: first_frame,
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(GATLING_SPRITE_SIZE)),
                    anchor: EntryEdge::Top.gatling_anchor(),
                    ..default()
                },
                transform: Transform::from_xyz(pos.x, pos.y, 0.5),
                ..default()
            },
            Enemy {
                health: phase.health,
                max_health: phase.health,
                state: EnemyState::Entering,
                radius: GATLING.radius,
                sprite_size: GATLING.sprite_size,
                anim_timer: Timer::from_seconds(MOTHERSHIP_ENTERING_DURATION, TimerMode::Once),
                phases: GATLING.phases,
                death_duration: GATLING.death_duration,
                death_shake_max: GATLING.death_shake_max,
                hit_sound: GATLING.hit_sound,
                death_explosion_sound: GATLING.death_explosion_sound,
                hit_flash_color: None,
            },
            GatlingMarker,
            GatlingBaseEdge(EntryEdge::Top),
            GatlingStartY(pos.y),
            GatlingEnteringAnim {
                timer: Timer::from_seconds(GATLING_ANIM_INTERVAL, TimerMode::Repeating),
                current_frame: 0,
            },
            PatternIndex(0),
            PatternTimer(Timer::from_seconds(0.0, TimerMode::Once)),
            DropTable {
                drops: &MOTHERSHIP_DROP_TABLE,
            },
        ));
    }
}

// ─── Gatling standalone Entering (sans Mothership) ─────────────────

fn gatling_standalone_entering(
    time: Res<Time>,
    mut query: Query<
        (&mut Enemy, &mut Transform, &GatlingStartY),
        (With<GatlingMarker>, Without<MothershipLink>),
    >,
) {
    for (mut enemy, mut transform, start_y) in query.iter_mut() {
        if enemy.state != EnemyState::Entering {
            continue;
        }

        enemy.anim_timer.tick(time.delta());
        let progress = enemy.anim_timer.fraction();
        let eased = 1.0 - (1.0 - progress).powi(2);
        transform.translation.y = start_y.0 - GATLING_ENTERING_DISTANCE * eased;

        if enemy.anim_timer.finished() {
            transform.translation.y = start_y.0 - GATLING_ENTERING_DISTANCE;
            enemy.state = EnemyState::Active(0);
        }
    }
}

// ─── Animation pendant Entering ────────────────────────────────────

fn gatling_entering_animate(
    time: Res<Time>,
    frames: Res<GatlingFrames>,
    mut query: Query<
        (
            &Enemy,
            &mut Handle<Image>,
            &mut GatlingEnteringAnim,
            &GatlingStyleComp,
        ),
        With<GatlingMarker>,
    >,
) {
    for (enemy, mut texture, mut anim, style) in query.iter_mut() {
        if enemy.state != EnemyState::Entering {
            continue;
        }
        // Pas d'animation de frames pour les sprites statiques
        if style.static_sprite {
            continue;
        }

        anim.timer.tick(time.delta());
        if anim.timer.just_finished() {
            anim.current_frame = (anim.current_frame + 1) % frames.0.len();
            *texture = frames.0[anim.current_frame].clone();
        }
    }
}

// ─── Pattern executor ──────────────────────────────────────────────

fn gatling_pattern_executor(
    time: Res<Time>,
    mut commands: Commands,
    frames: Res<GatlingFrames>,
    mut gatling_q: Query<
        (
            Entity,
            &Enemy,
            &Transform,
            &GatlingBaseEdge,
            &mut PatternTimer,
            &mut PatternIndex,
            &mut Handle<Image>,
            Option<&GatlingPatternOverride>,
            Option<&GatlingShoot>,
            Option<&GatlingFullAuto>,
        ),
        With<GatlingMarker>,
    >,
    player_q: Query<&Transform, (With<Player>, Without<GatlingMarker>)>,
) {
    for (
        entity,
        enemy,
        transform,
        base_edge,
        mut pattern_timer,
        mut pat_idx,
        mut texture,
        override_opt,
        shoot_opt,
        full_auto_opt,
    ) in gatling_q.iter_mut()
    {
        let phase_idx = match &enemy.state {
            EnemyState::Active(idx) => *idx,
            _ => continue,
        };

        pattern_timer.0.tick(time.delta());
        if !pattern_timer.0.just_finished() {
            continue;
        }

        let (pattern_name, pattern_duration, _pattern_count) = if let Some(ov) = override_opt {
            if ov.patterns.is_empty() {
                continue;
            }
            let idx = pat_idx.0 % ov.patterns.len();
            let p = &ov.patterns[idx];
            (p.name, p.duration, ov.patterns.len())
        } else {
            let phase = &enemy.phases[phase_idx];
            if phase.patterns.is_empty() {
                continue;
            }
            let idx = pat_idx.0 % phase.patterns.len();
            let p = &phase.patterns[idx];
            (p.name, p.duration, phase.patterns.len())
        };

        pat_idx.0 += 1;

        let next_duration = if let Some(ov) = override_opt {
            ov.patterns[pat_idx.0 % ov.patterns.len()].duration
        } else {
            let phase = &enemy.phases[phase_idx];
            phase.patterns[pat_idx.0 % phase.patterns.len()].duration
        };
        pattern_timer.0 = Timer::from_seconds(next_duration, TimerMode::Once);

        let prev_angle = if let Some(s) = shoot_opt {
            s.current_angle
        } else if let Some(fa) = full_auto_opt {
            fa.current_angle
        } else {
            0.0
        };

        match pattern_name {
            "aim_and_shoot" => {
                commands.entity(entity).remove::<GatlingFullAuto>();

                let cannon_dir = base_edge.0.enter_direction();
                let base_angle = cannon_dir.y.atan2(cannon_dir.x);

                let aim_dir = if let Ok(player_transform) = player_q.get_single() {
                    let diff =
                        player_transform.translation.truncate() - transform.translation.truncate();
                    if diff.length_squared() > 0.01 {
                        diff.normalize()
                    } else {
                        cannon_dir
                    }
                } else {
                    cannon_dir
                };

                let aim_angle = aim_dir.y.atan2(aim_dir.x);
                let mut relative_angle = aim_angle - base_angle;
                while relative_angle > std::f32::consts::PI {
                    relative_angle -= std::f32::consts::TAU;
                }
                while relative_angle < -std::f32::consts::PI {
                    relative_angle += std::f32::consts::TAU;
                }

                let max_rad = GATLING_AIM_MAX_ANGLE.to_radians();
                let clamped_angle = relative_angle.clamp(-max_rad, max_rad);

                commands.entity(entity).insert(GatlingShoot {
                    target_angle: clamped_angle,
                    current_angle: prev_angle,
                    elapsed: 0.0,
                    duration: pattern_duration,
                    anim_timer: Timer::from_seconds(
                        GATLING_SHOOT_ANIM_INTERVAL,
                        TimerMode::Repeating,
                    ),
                    current_frame: 0,
                    fired: false,
                    anim_started: false,
                    player_locked: false,
                });
            }
            "full_auto" => {
                commands.entity(entity).remove::<GatlingShoot>();

                let delay = 0.1 + fastrand::f32() * 1.2;
                let dir = if fastrand::bool() { 1.0 } else { -1.0 };

                commands.entity(entity).insert(GatlingFullAuto {
                    current_angle: prev_angle,
                    sweep_dir: dir,
                    startup_delay: delay,
                    elapsed: 0.0,
                    duration: pattern_duration,
                    fire_timer: Timer::from_seconds(
                        FULL_AUTO_FIRE_INTERVAL_START,
                        TimerMode::Repeating,
                    ),
                    anim_frame: None,
                    anim_timer: Timer::from_seconds(
                        FULL_AUTO_SHOOT_ANIM_INTERVAL,
                        TimerMode::Repeating,
                    ),
                });
            }
            "idle" => {
                commands.entity(entity).remove::<GatlingShoot>();
                commands.entity(entity).remove::<GatlingFullAuto>();
            }
            _ => {}
        }
    }
}

// ─── Mise à jour du pattern shoot ──────────────────────────────────

fn gatling_shoot_update(
    time: Res<Time>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    frames: Res<GatlingFrames>,
    mut query: Query<
        (
            Entity,
            &GatlingBaseEdge,
            &GatlingStyleComp,
            &mut GatlingShoot,
            &mut Transform,
            &mut Handle<Image>,
            Option<&GatlingAimBias>,
        ),
        With<GatlingMarker>,
    >,
    player_q: Query<&Transform, (With<Player>, Without<GatlingMarker>)>,
) {
    let dt = time.delta_seconds();
    let max_rad = GATLING_AIM_MAX_ANGLE.to_radians();
    let now = time.elapsed_seconds();

    for (_entity, base_edge, style, mut shoot, mut transform, mut texture, aim_bias) in
        query.iter_mut()
    {
        let (bias_center, bias_phase) = aim_bias
            .map(|b| (b.center_rad, b.phase_offset))
            .unwrap_or((0.0, 0.0));
        let min_rel = bias_center - max_rad;
        let max_rel = bias_center + max_rad;
        let rest_rot = base_edge.0.sprite_rotation();
        shoot.elapsed += dt;

        let anim_total_duration = frames.0.len() as f32 * GATLING_SHOOT_ANIM_INTERVAL;
        let anim_start_time = shoot.duration - anim_total_duration;

        // Suivi du joueur si proche, balayage sinon
        {
            let cannon_dir = base_edge.0.enter_direction();
            let base_atan2 = cannon_dir.y.atan2(cannon_dir.x);

            let mut player_locked = false;
            if let Ok(player_transform) = player_q.get_single() {
                let diff =
                    player_transform.translation.truncate() - transform.translation.truncate();
                let dist_sq = diff.length_squared();
                if dist_sq > 0.01 && dist_sq <= GATLING_AIM_DETECTION_RANGE.powi(2) {
                    let aim_atan2 = diff.y.atan2(diff.x);
                    let mut relative = aim_atan2 - base_atan2;
                    while relative > std::f32::consts::PI {
                        relative -= std::f32::consts::TAU;
                    }
                    while relative < -std::f32::consts::PI {
                        relative += std::f32::consts::TAU;
                    }
                    let clamped = relative.clamp(min_rel, max_rel);
                    // Verrou seulement si le joueur est dans le cône (décentré vers le centre MS)
                    if (relative - clamped).abs() < 1e-3 {
                        shoot.target_angle = clamped;
                        player_locked = true;
                    }
                }
            }

            if !player_locked {
                // Balayage sinusoïdal de la zone de recherche, autour du biais,
                // basé sur le temps global (oscillation continue indépendante du cycle de pattern).
                shoot.target_angle = bias_center
                    + max_rad * (now * GATLING_SWEEP_FREQUENCY + bias_phase).sin();
            }
            shoot.player_locked = player_locked;

            let speed_rad = GATLING_AIM_SPEED.to_radians() * dt;
            let angle_diff = shoot.target_angle - shoot.current_angle;
            let step = angle_diff.clamp(-speed_rad, speed_rad);
            shoot.current_angle += step;
            transform.rotation = rest_rot * Quat::from_rotation_z(shoot.current_angle);
        }

        // Animation de tir (seulement pour les sprites animés)
        if style.static_sprite {
            // Sprite statique : pas d'animation, tir à mi-durée,
            // uniquement si le joueur est verrouillé (sinon on continue de patrouiller).
            let fire_time = shoot.duration * 0.7;
            if !shoot.fired && shoot.elapsed >= fire_time && shoot.player_locked {
                shoot.fired = true;
                spawn_gatling_projectile(
                    &mut commands,
                    &asset_server,
                    &transform,
                    rest_rot,
                    shoot.current_angle,
                    style,
                );
            }
        } else {
            if !shoot.anim_started {
                if shoot.elapsed >= anim_start_time {
                    shoot.anim_started = true;
                    shoot.current_frame = 0;
                    shoot.anim_timer =
                        Timer::from_seconds(GATLING_SHOOT_ANIM_INTERVAL, TimerMode::Repeating);
                }
            } else {
                shoot.anim_timer.tick(time.delta());

                if shoot.anim_timer.just_finished() {
                    shoot.current_frame += 1;
                    if shoot.current_frame < frames.0.len() {
                        *texture = frames.0[shoot.current_frame].clone();
                    }
                }

                let fire_frame = frames.0.len() / 2;
                if !shoot.fired && shoot.current_frame >= fire_frame {
                    shoot.fired = true;
                    spawn_gatling_projectile(
                        &mut commands,
                        &asset_server,
                        &transform,
                        rest_rot,
                        shoot.current_angle,
                        style,
                    );
                }

                if shoot.current_frame >= frames.0.len() {
                    if let Some(frame) = frames.0.first() {
                        *texture = frame.clone();
                    }
                }
            }
        }
    }
}

/// Spawn un projectile de Gatling avec les paramètres du style.
fn spawn_gatling_projectile(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    transform: &Transform,
    rest_rot: Quat,
    current_angle: f32,
    style: &GatlingStyleComp,
) {
    let total_rot = rest_rot * Quat::from_rotation_z(current_angle);
    let local_cannon = Vec3::new(0.0, -1.0, 0.0);
    let shoot_dir_3 = total_rot.mul_vec3(local_cannon);
    let shoot_dir = Vec2::new(shoot_dir_3.x, shoot_dir_3.y);

    let cannon_tip = transform.translation.truncate() + shoot_dir * GATLING_SPRITE_SIZE;

    spawn_projectile(
        commands,
        asset_server,
        ProjectileSpawn {
            position: Vec3::new(cannon_tip.x, cannon_tip.y, 0.6),
            direction: shoot_dir,
            speed: style.projectile_speed,
            hitbox: HitboxShape::Circle(style.projectile_radius),
            team: Team::Enemy,
            damage: 1,
            sprite: ProjectileSprite::Colored {
                color: style.projectile_color,
                size: style.projectile_size,
            },
            death_folder: None,
        },
    );

    commands.spawn(AudioBundle {
        source: asset_server.load(style.shoot_sound),
        settings: PlaybackSettings::DESPAWN
            .with_volume(bevy::audio::Volume::new(style.shoot_sound_volume)),
    });
}

// ─── Mise à jour du pattern full_auto ──────────────────────────────

fn gatling_full_auto_update(
    time: Res<Time>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    frames: Res<GatlingFrames>,
    mut query: Query<
        (
            &GatlingBaseEdge,
            &GatlingStyleComp,
            &mut GatlingFullAuto,
            &mut Transform,
            &mut Handle<Image>,
        ),
        With<GatlingMarker>,
    >,
) {
    let dt = time.delta_seconds();
    let max_rad = GATLING_AIM_MAX_ANGLE.to_radians();

    for (base_edge, style, mut auto, mut transform, mut texture) in query.iter_mut() {
        let rest_rot = base_edge.0.sprite_rotation();

        // Délai de démarrage
        if auto.startup_delay > 0.0 {
            auto.startup_delay -= dt;
            transform.rotation = rest_rot * Quat::from_rotation_z(auto.current_angle);
            continue;
        }

        auto.elapsed += dt;

        // Courbe d'accélération
        let progress = (auto.elapsed / auto.duration).min(1.0);
        let ramp = progress.powf(FULL_AUTO_RAMP_FACTOR);

        let sweep_speed = FULL_AUTO_SWEEP_SPEED_START
            + (FULL_AUTO_SWEEP_SPEED_MAX - FULL_AUTO_SWEEP_SPEED_START) * ramp;
        let sweep_rad = sweep_speed.to_radians() * dt;

        let fire_interval = FULL_AUTO_FIRE_INTERVAL_START
            + (FULL_AUTO_FIRE_INTERVAL_MIN - FULL_AUTO_FIRE_INTERVAL_START) * ramp;
        auto.fire_timer
            .set_duration(std::time::Duration::from_secs_f32(fire_interval.max(0.05)));

        // Balayage ping-pong
        auto.current_angle += auto.sweep_dir * sweep_rad;
        if auto.current_angle >= max_rad {
            auto.current_angle = max_rad;
            auto.sweep_dir = -1.0;
        } else if auto.current_angle <= -max_rad {
            auto.current_angle = -max_rad;
            auto.sweep_dir = 1.0;
        }

        transform.rotation = rest_rot * Quat::from_rotation_z(auto.current_angle);

        // Animation de tir en cours (seulement pour sprites animés)
        if !style.static_sprite {
            if let Some(frame_idx) = auto.anim_frame {
                auto.anim_timer.tick(time.delta());
                if auto.anim_timer.just_finished() {
                    let next = frame_idx + 1;
                    if next < frames.0.len() {
                        auto.anim_frame = Some(next);
                        *texture = frames.0[next].clone();
                    } else {
                        auto.anim_frame = None;
                        if let Some(f) = frames.0.first() {
                            *texture = f.clone();
                        }
                    }
                }
            }
        }

        // Tir
        auto.fire_timer.tick(time.delta());
        if auto.fire_timer.just_finished() {
            if !style.static_sprite {
                auto.anim_frame = Some(0);
                auto.anim_timer =
                    Timer::from_seconds(FULL_AUTO_SHOOT_ANIM_INTERVAL, TimerMode::Repeating);
                if let Some(f) = frames.0.first() {
                    *texture = f.clone();
                }
            }

            spawn_gatling_projectile(
                &mut commands,
                &asset_server,
                &transform,
                rest_rot,
                auto.current_angle,
                style,
            );
        }
    }
}

// ─── Laser de visée ───────────────────────────────────────────────

/// Met à jour les lasers de visée : ligne fine du bout du canon vers le joueur.
fn gatling_laser_update(
    mut commands: Commands,
    gatling_q: Query<
        (&Transform, &GatlingBaseEdge, &Enemy, &GatlingStyleComp),
        With<GatlingMarker>,
    >,
    mut laser_q: Query<
        (Entity, &MothershipLink, &mut Transform, &mut Sprite),
        (With<GatlingLaser>, Without<GatlingMarker>),
    >,
    player_q: Query<&Transform, (With<Player>, Without<GatlingMarker>, Without<GatlingLaser>)>,
) {
    let player_pos = player_q.get_single().map(|t| t.translation.truncate()).ok();

    for (laser_entity, link, mut laser_tf, mut laser_sprite) in laser_q.iter_mut() {
        // Le laser est lié à un Gatling (mothership = gatling_entity)
        let Ok((gatling_tf, base_edge, enemy, style)) = gatling_q.get(link.mothership) else {
            // Gatling détruit → despawn le laser
            if let Some(mut e) = commands.get_entity(laser_entity) {
                e.despawn();
            }
            continue;
        };

        // Pas de laser si le gatling n'est pas actif
        if !matches!(enemy.state, EnemyState::Active(_)) || !style.has_laser {
            laser_sprite.color.set_a(0.0);
            continue;
        }

        // Position du bout du canon (dans la direction réelle de visée du sprite)
        let rest_rot = base_edge.0.sprite_rotation();
        let total_rot = gatling_tf.rotation;
        let local_cannon = Vec3::new(0.0, -1.0, 0.0);
        let shoot_dir_3 = total_rot.mul_vec3(local_cannon);
        let cannon_dir = Vec2::new(shoot_dir_3.x, shoot_dir_3.y);

        let cannon_tip = gatling_tf.translation.truncate() + cannon_dir * GATLING_SPRITE_SIZE;

        // Le laser suit toujours la direction du canon (sweep ou lock)
        // et continue jusqu'au hors-écran : longueur fixe largement supérieure.
        let cannon_angle = cannon_dir.y.atan2(cannon_dir.x) - std::f32::consts::FRAC_PI_2;

        laser_tf.translation = Vec3::new(cannon_tip.x, cannon_tip.y, 0.49);
        laser_tf.rotation = Quat::from_rotation_z(cannon_angle);

        laser_sprite.custom_size = Some(Vec2::new(2.0, 2000.0));
        laser_sprite.color = style.laser_color;
        let _ = (rest_rot, player_pos);
    }
}
