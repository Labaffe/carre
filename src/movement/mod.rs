pub mod movement;
pub mod movements;
pub mod sinusoid;
pub mod translate;
pub mod goto;
use bevy::prelude::*;
use crate::movement::movements::Movements;
use crate::GameState;
use crate::movement::movement::Movement;
pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_systems(
            FixedUpdate, 
            movement_driver.run_if(in_state(GameState::Editor))
        );
    }
}

pub fn movement_driver(
    time:Res<Time>,
    mut query:Query<(Entity,&mut Movements,&mut Transform)>
) {
    for (entity,mut sinusoid,mut transform) in query.iter_mut() {
        let movement = sinusoid.get_movement(transform.translation.xy(),time.delta(),);
        transform.translation.x += movement.x;
        transform.translation.y += movement.y;
    }
}