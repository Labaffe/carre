//! Système de missiles : tir, mouvement, collision avec les astéroïdes.
//!
//! Le pattern de tir et la hitbox sont définis par la `WeaponDef` équipée sur le joueur.
//! Supporte les hitbox circulaires (Standard Missile) et rectangulaires orientées (Red Projectile).
//! Un HashSet empêche les doubles despawn quand plusieurs missiles touchent la même cible en une frame.

use crate::asteroid::{Asteroid, HitFlash};
use crate::crosshair::Crosshair;
use crate::difficulty::Difficulty;
use crate::explosion::{spawn_explosion, spawn_projectile_death};
use crate::player::Player;
use crate::weapon::{HitboxShape, Weapon};
use bevy::prelude::*;

pub struct MissilePlugin;

impl Plugin for MissilePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(FireRateTimer(Timer::from_seconds(0.2, TimerMode::Repeating)))
            .add_systems(OnEnter(crate::state::GameState::Playing), reset_fire_rate)
            .add_systems(
                Update,
                (shoot, move_missiles, missile_asteroid_collision)
                    .run_if(in_state(crate::state::GameState::Playing)),
            );
    }
}

#[derive(Resource)]
struct FireRateTimer(Timer);

fn reset_fire_rate(mut timer: ResMut<FireRateTimer>) {
    timer.0.reset();
}

// ─── Composant Missile ───────────────────────────────────────────────

#[derive(Component)]
pub struct Missile {
    velocity: Vec3,
    pub hitbox: HitboxShape,
    /// Dossier optionnel de frames de mort du projectile.
    pub death_folder: Option<&'static str>,
}

// ─── Collision OBB vs Cercle ─────────────────────────────────────────

/// Test de collision rectangle orienté (OBB) vs cercle.
/// Projette le centre du cercle dans le repère local du rectangle,
/// puis trouve le point le plus proche sur le rectangle.
fn obb_circle_collision(
    rect_pos: Vec2,
    rect_angle: f32,
    half_length: f32,
    half_width: f32,
    circle_pos: Vec2,
    circle_radius: f32,
) -> bool {
    let delta = circle_pos - rect_pos;
    let cos = rect_angle.cos();
    let sin = rect_angle.sin();
    let local_x = delta.dot(Vec2::new(cos, sin));
    let local_y = delta.dot(Vec2::new(-sin, cos));

    let cx = local_x.clamp(-half_width, half_width);
    let cy = local_y.clamp(-half_length, half_length);

    (local_x - cx).powi(2) + (local_y - cy).powi(2) <= circle_radius * circle_radius
}

/// Teste la collision entre un missile (hitbox variable) et un cercle (astéroïde).
fn missile_hits_circle(
    missile_pos: Vec2,
    missile_rot: Quat,
    hitbox: &HitboxShape,
    circle_pos: Vec2,
    circle_radius: f32,
) -> bool {
    match hitbox {
        HitboxShape::Circle(r) => {
            missile_pos.distance(circle_pos) < *r + circle_radius
        }
        HitboxShape::Rect { half_length, half_width } => {
            let angle = missile_rot.to_euler(EulerRot::ZYX).0;
            obb_circle_collision(
                missile_pos, angle,
                *half_length, *half_width,
                circle_pos, circle_radius,
            )
        }
    }
}

// ─── Tir ─────────────────────────────────────────────────────────────

/// Rotation 2D d'un vecteur direction par un angle en radians.
fn rotate_direction(dir: Vec2, angle: f32) -> Vec2 {
    let cos = angle.cos();
    let sin = angle.sin();
    Vec2::new(dir.x * cos - dir.y * sin, dir.x * sin + dir.y * cos)
}

fn shoot(
    mouse: Res<ButtonInput<MouseButton>>,
    mut fire_timer: ResMut<FireRateTimer>,
    time: Res<Time>,
    player_q: Query<(&Transform, &Weapon), With<Player>>,
    crosshair_q: Query<&Transform, With<Crosshair>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    if !mouse.pressed(MouseButton::Left) {
        return;
    }

    let Ok((player_transform, weapon)) = player_q.get_single() else {
        return;
    };
    let Ok(crosshair_transform) = crosshair_q.get_single() else {
        return;
    };

    // Adapter la cadence de tir à l'arme actuelle
    fire_timer.0.set_duration(std::time::Duration::from_secs_f32(weapon.def.fire_rate));
    fire_timer.0.tick(time.delta());
    if !fire_timer.0.just_finished() {
        return;
    }

    let player_pos = player_transform.translation;
    let crosshair_pos = crosshair_transform.translation;
    let base_dir = (crosshair_pos - player_pos).truncate().normalize_or_zero();

    if base_dir == Vec2::ZERO {
        return;
    }

    let def = &weapon.def;
    let origin = Vec3::new(player_pos.x, player_pos.y, -0.1); // z négatif = derrière le joueur

    // Spawn un projectile par angle dans le pattern
    for shot in def.pattern.iter() {
        let dir = rotate_direction(base_dir, shot.0);
        let angle = dir.y.atan2(dir.x) - std::f32::consts::FRAC_PI_2;

        commands.spawn((
            SpriteBundle {
                texture: asset_server.load(def.texture_path),
                transform: Transform {
                    translation: origin,
                    rotation: Quat::from_rotation_z(angle),
                    ..default()
                },
                ..default()
            },
            Missile {
                velocity: dir.extend(0.0) * def.speed,
                hitbox: def.hitbox.clone(),
                death_folder: def.death_folder,
            },
        ));
    }

    commands.spawn(AudioBundle {
        source: asset_server.load("audio/projectile.ogg"),
        settings: PlaybackSettings::DESPAWN,
    });
}

// ─── Collision ───────────────────────────────────────────────────────

fn missile_asteroid_collision(
    mut commands: Commands,
    missile_q: Query<(Entity, &Transform, &Missile)>,
    mut asteroid_q: Query<(Entity, &Transform, &mut Asteroid)>,
    asset_server: Res<AssetServer>,
    difficulty: Res<Difficulty>,
) {
    let mut despawned_missiles = std::collections::HashSet::new();
    let mut despawned_asteroids = std::collections::HashSet::new();

    for (missile_entity, missile_transform, missile) in missile_q.iter() {
        if despawned_missiles.contains(&missile_entity) {
            continue;
        }
        for (asteroid_entity, asteroid_transform, mut asteroid) in asteroid_q.iter_mut() {
            if despawned_asteroids.contains(&asteroid_entity) {
                continue;
            }

            let hit = missile_hits_circle(
                missile_transform.translation.truncate(),
                missile_transform.rotation,
                &missile.hitbox,
                asteroid_transform.translation.truncate(),
                asteroid.radius,
            );

            if hit {
                spawn_projectile_death(
                    &mut commands,
                    &asset_server,
                    missile_transform.translation,
                    missile.death_folder,
                );
                commands.entity(missile_entity).despawn();
                despawned_missiles.insert(missile_entity);
                asteroid.health -= 1;

                if asteroid.health <= 0 {
                    if !despawned_asteroids.contains(&asteroid_entity) {
                        spawn_explosion(
                            &mut commands,
                            &asset_server,
                            asteroid_transform.translation,
                            asteroid.size,
                            asteroid.texture_index,
                            asteroid.base_velocity * difficulty.factor,
                            asteroid_transform.rotation,
                        );
                        commands.spawn(AudioBundle {
                            source: asset_server.load("audio/asteroid_die.ogg"),
                            settings: PlaybackSettings::DESPAWN,
                        });
                        commands.entity(asteroid_entity).despawn();
                        despawned_asteroids.insert(asteroid_entity);
                    }
                } else {
                    commands.entity(asteroid_entity)
                        .insert(HitFlash(Timer::from_seconds(0.06, TimerMode::Once)));
                    commands.spawn(AudioBundle {
                        source: asset_server.load("audio/asteroid_hit.ogg"),
                        settings: PlaybackSettings::DESPAWN,
                    });
                }
                break;
            }
        }
    }
}

// ─── Mouvement ───────────────────────────────────────────────────────

/// Déplace les missiles et les supprime quand ils sortent de l'écran.
fn move_missiles(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &Missile)>,
    time: Res<Time>,
) {
    for (entity, mut transform, missile) in query.iter_mut() {
        transform.translation += missile.velocity * time.delta_seconds();

        let p = transform.translation;
        if p.x.abs() > 1200.0 || p.y.abs() > 900.0 {
            commands.entity(entity).despawn();
        }
    }
}
