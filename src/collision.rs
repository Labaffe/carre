//! Collision joueur ↔ astéroïde.
//! Quand le joueur touche un astéroïde, c'est le game over :
//! le joueur et l'astéroïde sont supprimés, l'état passe à GameOver.

use crate::asteroid::Asteroid;
use crate::debug::DebugMode;
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

/// Rayon de la hitbox du joueur (sprite 128x128, hitbox ~70% du demi-côté).
pub const PLAYER_RADIUS: f32 = 45.0;

fn player_asteroid_collision(
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    player_q: Query<(Entity, &Transform), With<Player>>,
    asteroid_q: Query<(Entity, &Transform, &Asteroid)>,
    debug: Res<DebugMode>,
) {
    if debug.0 {
        return;
    }

    let Ok((player_entity, player_transform)) = player_q.get_single() else {
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
) {
    commands.entity(player_entity).despawn_recursive();
    commands.entity(asteroid_entity).despawn();
    next_state.set(GameState::GameOver);
}
