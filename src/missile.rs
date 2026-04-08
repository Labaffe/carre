use bevy::prelude::*;
use crate::crosshair::Crosshair;
use crate::player::Player;
use crate::state::GameState;

pub struct MissilePlugin;

impl Plugin for MissilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (shoot, move_missiles).run_if(in_state(GameState::Playing)),
        );
    }
}

#[derive(Component)]
pub struct Missile {
    velocity: Vec3,
}

fn shoot(
    mouse: Res<ButtonInput<MouseButton>>,
    player_q: Query<&Transform, With<Player>>,
    crosshair_q: Query<&Transform, With<Crosshair>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(player_transform) = player_q.get_single() else { return; };
    let Ok(crosshair_transform) = crosshair_q.get_single() else { return; };

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
