use bevy::prelude::*;
use crate::GameState;


#[derive(Component)]
pub struct HitFlash(pub Timer);
pub fn animate_hit_flash(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Sprite, &mut HitFlash)>,
) {
    for (entity, mut sprite, mut flash) in query.iter_mut() {
        flash.0.tick(time.delta());

        if flash.0.finished() {
            sprite.color = Color::WHITE;
            commands.entity(entity).remove::<HitFlash>();
        } else {
            // Multiplie chaque canal par une valeur très élevée → surexpose le sprite en blanc pur
            sprite.color = Color::srgba(100.0, 100.0, 100.0, 1.0);
        }
    }
}