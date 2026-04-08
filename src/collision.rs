use crate::asteroid::Asteroid;
use crate::player::Player;
use crate::state::GameState;
use crate::thruster::Thruster;
use bevy::prelude::*;

pub struct CollisionPlugin;

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            player_asteroid_collision.run_if(in_state(GameState::Playing)),
        );
    }
}

pub const PLAYER_RADIUS: f32 = 29.0;

fn player_asteroid_collision(
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    player_q: Query<(Entity, &Transform), With<Player>>,
    asteroid_q: Query<(Entity, &Transform, &Asteroid)>,
    thruster_q: Query<Entity, With<Thruster>>,
) {
    let Ok((player_entity, player_transform)) = player_q.get_single() else {
        return;
    };

    let Ok(thruster_entity) = thruster_q.get_single() else {
        return;
    };
    for (asteroid_entity, asteroid_transform, asteroid) in asteroid_q.iter() {
        let distance = player_transform
            .translation
            .distance(asteroid_transform.translation);

        if distance < PLAYER_RADIUS + asteroid.radius {
            game_over(
                &mut commands,
                &mut next_state,
                player_entity,
                asteroid_entity,
                thruster_entity,
            );
            break;
        }
    }
}

fn game_over(
    commands: &mut Commands,
    next_state: &mut ResMut<NextState<GameState>>,
    player_entity: Entity,
    asteroid_entity: Entity,
    thruster_entity: Entity,
) {
    commands.entity(player_entity).despawn();
    commands.entity(thruster_entity).despawn();
    commands.entity(asteroid_entity).despawn();
    next_state.set(GameState::GameOver);
}
