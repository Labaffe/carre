use crate::asteroid::{Asteroid, HitFlash};
use crate::crosshair::Crosshair;
use crate::explosion::spawn_explosion;
use crate::player::Player;
use crate::weapon::Weapon;
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

#[derive(Component)]
pub struct Missile {
    velocity: Vec3,
    /// Demi-longueur de la hitbox (axe du missile).
    pub half_length: f32,
    /// Demi-largeur de la hitbox (perpendiculaire).
    pub half_width: f32,
}

/// Test de collision rectangle orienté (OBB) vs cercle.
/// Projette le centre du cercle dans le repère local du rectangle,
/// puis trouve le point le plus proche sur le rectangle.
fn obb_circle_collision(
    rect_pos: Vec3,
    rect_rot: Quat,
    half_length: f32,
    half_width: f32,
    circle_pos: Vec3,
    circle_radius: f32,
) -> bool {
    // Vecteur du rectangle vers le cercle
    let delta = circle_pos - rect_pos;
    let delta2 = delta.truncate();

    // Axes locaux du rectangle (le missile pointe vers le haut local = Y local)
    let (axis_x, axis_y) = {
        let angle = rect_rot.to_euler(EulerRot::ZYX).0;
        let cos = angle.cos();
        let sin = angle.sin();
        // X local = droite du missile, Y local = avant du missile
        (Vec2::new(cos, sin), Vec2::new(-sin, cos))
    };

    // Projection dans le repère local
    let local_x = delta2.dot(axis_x);
    let local_y = delta2.dot(axis_y);

    // Point le plus proche sur le rectangle
    let closest_x = local_x.clamp(-half_width, half_width);
    let closest_y = local_y.clamp(-half_length, half_length);

    let dist_sq = (local_x - closest_x).powi(2) + (local_y - closest_y).powi(2);
    dist_sq <= circle_radius * circle_radius
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
    let direction = (crosshair_pos - player_pos).truncate().normalize_or_zero();

    if direction == Vec2::ZERO {
        return;
    }

    let angle = direction.y.atan2(direction.x) - std::f32::consts::FRAC_PI_2;
    let def = &weapon.def;

    // Missile central
    let pos = Vec3::new(player_pos.x, player_pos.y, -0.1);
    spawn_missile(&mut commands, asset_server.load(def.texture_path), pos, direction, angle, def.speed, def.hitbox_half_length, def.hitbox_half_width);

    // Missiles latéraux si l'arme en a
    if def.projectile_count >= 3 {
        let perpendicular = Vec2::new(-direction.y, direction.x);
        let offset = def.side_offset;

        let pos_left = Vec3::new(
            player_pos.x + perpendicular.x * offset,
            player_pos.y + perpendicular.y * offset,
            -0.1,
        );
        let pos_right = Vec3::new(
            player_pos.x - perpendicular.x * offset,
            player_pos.y - perpendicular.y * offset,
            -0.1,
        );

        spawn_missile(&mut commands, asset_server.load(def.texture_path), pos_left, direction, angle, def.speed, def.hitbox_half_length, def.hitbox_half_width);
        spawn_missile(&mut commands, asset_server.load(def.texture_path), pos_right, direction, angle, def.speed, def.hitbox_half_length, def.hitbox_half_width);
    }

    // son de tir
    commands.spawn(AudioBundle {
        source: asset_server.load("audio/projectile.ogg"),
        settings: PlaybackSettings::DESPAWN,
    });
}

fn spawn_missile(
    commands: &mut Commands,
    texture: Handle<Image>,
    position: Vec3,
    direction: Vec2,
    angle: f32,
    speed: f32,
    half_length: f32,
    half_width: f32,
) {
    commands.spawn((
        SpriteBundle {
            texture,
            transform: Transform {
                translation: position,
                rotation: Quat::from_rotation_z(angle),
                ..default()
            },
            ..default()
        },
        Missile {
            velocity: direction.extend(0.0) * speed,
            half_length,
            half_width,
        },
    ));
}

fn missile_asteroid_collision(
    mut commands: Commands,
    missile_q: Query<(Entity, &Transform, &Missile)>,
    mut asteroid_q: Query<(Entity, &Transform, &mut Asteroid)>,
    asset_server: Res<AssetServer>,
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

            let hit = obb_circle_collision(
                missile_transform.translation,
                missile_transform.rotation,
                missile.half_length,
                missile.half_width,
                asteroid_transform.translation,
                asteroid.radius,
            );

            if hit {
                commands.entity(missile_entity).despawn();
                despawned_missiles.insert(missile_entity);
                asteroid.health -= 1;

                if asteroid.health <= 0 {
                    spawn_explosion(
                        &mut commands,
                        &asset_server,
                        asteroid_transform.translation,
                        asteroid.size,
                    );
                    commands.spawn(AudioBundle {
                        source: asset_server.load("audio/asteroid_die.ogg"),
                        settings: PlaybackSettings::DESPAWN,
                    });
                    commands.entity(asteroid_entity).despawn();
                    despawned_asteroids.insert(asteroid_entity);
                } else {
                    commands.entity(asteroid_entity)
                        .insert(HitFlash(Timer::from_seconds(0.25, TimerMode::Once)));
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
