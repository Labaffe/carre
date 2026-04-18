
use bevy::prelude::*;
use crate::tweening::systems::{tween_system};

pub struct UiTweenPlugin;

impl Plugin for UiTweenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            tween_system::<Transform>,
            tween_system::<Style>,
            tween_system::<BackgroundColor>,
        ));
    }
}