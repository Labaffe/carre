use bevy::prelude::*;
use crate::GameState;
pub struct DespawnZonePlugin;

impl Plugin for DespawnZonePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(
                Update,
                ( despawn_on_trigger)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

#[derive(Component)]
pub struct DespawnZone {
    pub x:f32,
    pub y:f32,
    pub width:f32,
    pub height:f32
}
fn despawn_on_trigger(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, & Transform, &mut DespawnZone)>,
) {
    for (entity, mut transform, mut zone) in query.iter_mut() {
        let trigger = (transform.translation.x > zone.x)
            & (transform.translation.x < zone.x + zone.width)
            & (transform.translation.y > zone.y)
            & (transform.translation.y < zone.y + zone.height);
        if trigger {
            if let Ok(mut e) = commands.get_entity(entity) { e.despawn(); }
        }
    }
}