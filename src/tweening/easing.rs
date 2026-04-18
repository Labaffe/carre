use bevy::prelude::*;

/* =========================
   EASING
   ========================= */

#[derive(Clone, Copy)]
pub enum Ease {
    Linear,
    InQuad,
    OutQuad,
}

impl Ease {
    pub fn sample(self, t: f32) -> f32 {
        match self {
            Ease::Linear => t,
            Ease::InQuad => t * t,
            Ease::OutQuad => t * (2.0 - t),
        }
    }
}