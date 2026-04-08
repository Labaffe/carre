use crate::asteroid::Asteroid;
use crate::player::Player;
use crate::state::GameState;
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

const PLAYER_RADIUS: f32 = 32.0;

fn player_asteroid_collision(
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    player_q: Query<(Entity, &Transform), With<Player>>,
    asteroid_q: Query<(Entity, &Transform, &Asteroid)>,
) {
    let Ok((player_entity, player_transform)) = player_q.get_single() else {
        return;
    };

    for (asteroid_entity, asteroid_transform, asteroid) in asteroid_q.iter() {
        let distance = player_transform
            .translation
            .distance(asteroid_transform.translation);

        if distance < PLAYER_RADIUS + asteroid.radius {
            commands.entity(player_entity).despawn();
            commands.entity(asteroid_entity).despawn();
            next_state.set(GameState::GameOver);
            break;
        }
    }
}
