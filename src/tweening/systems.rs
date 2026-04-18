use bevy::prelude::*;
use crate::tweening::tween::{ TweenSequence};
use crate::tweening::component::{TweenTarget};

pub fn tween_system<T: TweenTarget>(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut TweenSequence<T>, &mut T)>,
) {
    for (entity, mut seq, mut target) in query.iter_mut() {
        let dt = time.delta_seconds();

        seq.current.tick(dt);

        let value = seq.current.value();
        T::apply(value, &mut target);

        if seq.current.finished() {
            if let Some(next) = seq.queue.pop() {
                seq.current = next;
            } else {
                commands.entity(entity).remove::<TweenSequence<T>>();
            }
        }
    }
}
