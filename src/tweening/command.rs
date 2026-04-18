use bevy::prelude::*;
use 

pub trait TweenCommandsExt {
    fn tween_translation_x(
        &mut self,
        entity: Entity,
        tween: Tween,
    ) -> &mut Self;
}

impl TweenCommandsExt for Commands<'_, '_> {
    fn tween_translation_x(
        &mut self,
        entity: Entity,
        tween: Tween,
    ) -> &mut Self {
        self.entity(entity).insert((
            TweenSequence::<Transform>::new(tween),
        ));
        self
    }
}