use bevy::prelude::*;
pub trait TweenTarget: Component {
    fn apply(value: f32, target: &mut Self);
}

/* =========================
   COMPONENTS
   ========================= */

#[derive(Component)]
pub struct TweenTranslation;

#[derive(Component)]
pub struct TweenScale;

#[derive(Component)]
pub struct TweenOpacity;

#[derive(Component)]
pub struct TweenUIPosX;

#[derive(Component)]
pub struct TweenUIPosY;

/* =========================
   TARGET IMPLEMENTATIONS
   ========================= */

impl TweenTarget for Transform {
    fn apply(value: f32, target: &mut Self) {
        target.translation.x = value;
    }
}

impl TweenTarget for Style {
    fn apply(value: f32, target: &mut Self) {
        target.left = Val::Px(value);
    }
}

impl TweenTarget for BackgroundColor {
    fn apply(value: f32, target: &mut Self) {
        target.0.set_a(value);
    }
}