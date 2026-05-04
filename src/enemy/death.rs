use bevy::prelude::*;
use crate::{GameState, enemy::enemy::Enemy, physic::health::Health};
pub struct EnemyDeathPlugin;

impl Plugin for EnemyDeathPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(
                Update,
                ( detect_death)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

#[derive(Component)]
pub struct DeathAnim {
    pub x:f32,
    pub y:f32,
    pub width:f32,
    pub height:f32
}


fn detect_death(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &Health, &Enemy)>,
) {
    for (entity, mut health, mut enemy) in query.iter_mut() {
        if health.is_dead() & !health.dying {
            health.dying = true;
        }
    }
}