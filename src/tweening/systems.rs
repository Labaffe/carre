use bevy::prelude::*;
use crate::tweening::component::*;
use crate::tweening::tween::{TweenSequence};
/* =========================
   GENERIC TWEEN SYSTEM
   ========================= */

pub fn tween_system<T: TweenTarget>(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut TweenSequence<T>, &mut T::Component)>,
) {
    for (entity, mut seq, mut target) in query.iter_mut() {
        let dt = time.delta_seconds();

        // advance time
        seq.current.tick(dt);

        // apply value
        let value = seq.current.value();
        T::apply(value, &mut target);

        // handle end
        if seq.current.finished() {
            if let Some(next) = seq.queue.pop() {
                seq.current = next;
            } else {
                commands.entity(entity).remove::<TweenSequence<T>>();
            }
        }
    }
}