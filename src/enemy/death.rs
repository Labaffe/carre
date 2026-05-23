use bevy::prelude::*;
use crate::{GameState, behavior::{self, behavior::BehaviorComponent, choice_list::TransitionMessages}, enemy::enemy::{Enemy, EnemyDeathEvent}, item::item::{DropEvent, DropTable}, physic::health::Health};
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

/// Marker inséré une fois sur les entités qui ont déclenché leur "die"
/// message. Empêche `detect_death` de re-fire (sinon le behavior tree
/// retransiterait à chaque frame tant que HP=0 et entité pas encore despawn).
#[derive(Component)]
pub struct Dying;

pub fn detect_death(
    mut commands: Commands,
    time: Res<Time>,
    mut drop_events: MessageWriter<DropEvent>,
    mut death_events: MessageWriter<EnemyDeathEvent>,
    mut query: Query<(Entity, &Health, &Enemy, &Transform, Option<&DropTable>, &mut TransitionMessages), Without<Dying>>,
) {
    for (entity, health, _enemy, transform, drop_table, mut messages) in query.iter_mut() {
        if health.is_dead() {
            if let Some(table) = drop_table {
                drop_events.write(DropEvent {
                    position: transform.translation,
                    table: table.drops,
                });
            }
            messages.messages.push("die".to_string());
            death_events.write(EnemyDeathEvent {
                entity,
                position: transform.translation,
            });
            if let Ok(mut e) = commands.get_entity(entity) {
                // `try_insert` : safe si l'entité est despawn entre `get_entity`
                // et le flush des commands (ex: tuée la même frame par une bombe).
                e.try_insert(Dying);
            }
        }
    }
}

pub fn despawn(mut commands: Commands,mut death_events: MessageWriter<EnemyDeathEvent>,query:Query<Entity,With<DespawnSelf>>) {
    for entity in query.iter() {
        if let Ok(mut e) = commands.get_entity(entity) { e.try_despawn(); }
    }
}