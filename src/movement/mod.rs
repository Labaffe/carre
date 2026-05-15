pub mod movement;
pub mod movements;
pub mod sinusoid;
pub mod translate;
pub mod chase;
pub mod rush;
pub mod goto;
pub mod shake;
pub mod rotate;
use bevy::prelude::*;
use crate::movement::movements::Movements;
use crate::GameState;
use crate::movement::movement::Movement;
use crate::player::player::Player;
pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_systems(
            FixedUpdate, 
            movement_driver.run_if(in_state(GameState::Playing))
        );
    }
}

pub fn movement_driver(
    time: Res<Time>,
    mut query: Query<(Entity, &mut Movements, &mut Transform)>,
    query_player: Query<(&Player, &Transform), Without<Movements>>,
) {
    // Gracefully handle missing or multiple players
    let Ok((_, player_transform)) = query_player.get_single() else {
        return;
    };

    let player_pos = player_transform.translation.xy();

    for (_, mut movements, mut transform) in query.iter_mut() {
        let movement = movements.get_movement(
            transform.translation.xy(),
            time.delta(),
            player_pos,
        );
        transform.translation.x += movement.x;
        transform.translation.y += movement.y;
    }
}