use bevy::prelude::*;
use crate::{GameState, enemy::enemy::{Enemy, EnemyDeathEvent}, item::item::{DropEvent, DropTable}, physic::health::Health};
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
#[derive(Component,Clone)]
pub struct DespawnSelf;

fn detect_death(
    mut commands: Commands,
    time: Res<Time>,
    mut drop_events: EventWriter<DropEvent>,
    mut death_events: EventWriter<EnemyDeathEvent>,
    mut query: Query<(Entity, &mut Health, &Enemy, &Transform, Option<&DropTable>)>,
) {
    for (entity, mut health, mut enemy,transform,drop_table) in query.iter_mut() {
        if health.is_dead() & !health.dying {
            health.dying = true;
            if let Some(table) = drop_table {
            drop_events.send(DropEvent {
                position: transform.translation,
                table: table.drops,
            });
            }
            death_events.send(EnemyDeathEvent {
                entity,
                position: transform.translation,
            });
        }
    }
}