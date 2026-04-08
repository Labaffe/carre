use crate::asteroid::Asteroid;
use crate::crosshair::Crosshair;
use crate::explosion::spawn_explosion;
use crate::player::Player;
use bevy::prelude::*;

pub struct MissilePlugin;

impl Plugin for MissilePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(FireRate::default())
            .add_systems(OnEnter(crate::state::GameState::Playing), reset_fire_rate)
            .add_systems(
                Update,
                (shoot, move_missiles, missile_asteroid_collision)
                    .run_if(in_state(crate::state::GameState::Playing)),
            );
    }
}

/// Cadence de tir : 1 tir toutes les 0.2 secondes (5 tirs/s).
#[derive(Resource)]
struct FireRate(Timer);

impl Default for FireRate {
    fn default() -> Self {
        Self(Timer::from_seconds(0.2, TimerMode::Repeating))
    }
}

fn reset_fire_rate(mut fire_rate: ResMut<FireRate>) {
    fire_rate.0.reset();
}

#[derive(Component)]
pub struct Missile {
    velocity: Vec3,
}

fn shoot(
    mouse: Res<ButtonInput<MouseButton>>,
    mut fire_rate: ResMut<FireRate>,
    time: Res<Time>,
    player_q: Query<&Transform, With<Player>>,
    crosshair_q: Query<&Transform, With<Crosshair>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    if !mouse.pressed(MouseButton::Left) {
        return;
    }

    fire_rate.0.tick(time.delta());
    if !fire_rate.0.just_finished() {
        return;
    }

    let Ok(player_transform) = player_q.get_single() else {
        return;
    };
    let Ok(crosshair_transform) = crosshair_q.get_single() else {
        return;
    };

    let player_pos = player_transform.translation;
    let crosshair_pos = crosshair_transform.translation;
    let direction = (crosshair_pos - player_pos).truncate().normalize_or_zero();

    if direction == Vec2::ZERO {
        return;
    }

    let angle = direction.y.atan2(direction.x) - std::f32::consts::FRAC_PI_2;

    commands.spawn((
        SpriteBundle {
            texture: asset_server.load("images/missile.png"),
            transform: Transform {
                translation: Vec3::new(player_pos.x, player_pos.y, 0.5),
                rotation: Quat::from_rotation_z(angle),
                ..default()
            },
            ..default()
        },
        Missile {
            velocity: direction.extend(0.0) * 600.0,
        },
    ));

    // son de tir
    commands.spawn(AudioBundle {
        source: asset_server.load("audio/projectile.ogg"),
        settings: PlaybackSettings::DESPAWN,
    });
}

const MISSILE_RADIUS: f32 = 6.0;

fn missile_asteroid_collision(
    mut commands: Commands,
    missile_q: Query<(Entity, &Transform), With<Missile>>,
    mut asteroid_q: Query<(Entity, &Transform, &mut Asteroid)>,
    asset_server: Res<AssetServer>,
) {
    for (missile_entity, missile_transform) in missile_q.iter() {
        for (asteroid_entity, asteroid_transform, mut asteroid) in asteroid_q.iter_mut() {
            let distance = missile_transform
                .translation
                .distance(asteroid_transform.translation);

            if distance < MISSILE_RADIUS + asteroid.radius {
                commands.entity(missile_entity).despawn();
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
                } else {
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
