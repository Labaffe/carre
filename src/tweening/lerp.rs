use bevy::prelude::*;

pub trait Lerp {
    fn lerp(&self, other: &Self, t: f32) -> Self;
}

impl Lerp for f32 {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        self + (other - self) * t
    }
}

impl Lerp for Vec3 {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        *self + (*other - *self) * t
    }
}
impl Lerp for Vec2 {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        *self + (*other - *self) * t
    }
}