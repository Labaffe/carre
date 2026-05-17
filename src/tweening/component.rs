use bevy::prelude::*;
use bevy::color::Alpha;
pub trait TweenTarget: Send + Sync + 'static {
    type Component: bevy::ecs::component::Component;

    fn apply(value: f32, target: &mut Self::Component);
}

/* =========================
   TARGET MARKERS
   ========================= */

// Transform
pub struct TranslationX;
pub struct TranslationY;

// UI Node
pub struct StyleLeft;
pub struct StyleTop;

// Opacity
pub struct UiOpacity;

/* =========================
   IMPLEMENTATIONS
   ========================= */

// -------- Transform --------

impl TweenTarget for TranslationX {
    type Component = Transform;

    fn apply(value: f32, target: &mut Transform) {
        target.translation.x = value;
    }
}

impl TweenTarget for TranslationY {
    type Component = Transform;

    fn apply(value: f32, target: &mut Transform) {
        target.translation.y = value;
    }
}

// -------- UI Node --------

impl TweenTarget for StyleLeft {
    type Component = Node;

    fn apply(value: f32, target: &mut Node) {
        target.left = Val::Px(value);
    }
}

impl TweenTarget for StyleTop {
    type Component = Node;

    fn apply(value: f32, target: &mut Node) {
        target.top = Val::Px(value);
    }
}

// -------- Opacity --------

impl TweenTarget for UiOpacity {
    type Component = BackgroundColor;

    fn apply(value: f32, target: &mut BackgroundColor) {
        target.0.set_alpha(value);
    }
}