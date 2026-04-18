
use bevy::prelude::*;
use crate::tweening::systems::{tween_system};
use crate::tweening::component::*;
pub struct UiTweenPlugin;

impl Plugin for UiTweenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            tween_system::<TranslationX>,
            tween_system::<TranslationY>,
            tween_system::<StyleLeft>,
            tween_system::<StyleTop>,
            tween_system::<UiOpacity>,
        ));
    }
}