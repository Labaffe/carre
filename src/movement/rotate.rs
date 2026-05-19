use bevy::prelude::*;
use crate::movement::Movement;
use std::time::Duration;

/// Rotation autour d'un point fixe. `freq` est en **tours par seconde**.
/// Vitesse tangentielle = `r * 2π * freq` (loi du mouvement circulaire).
/// Retourne `ZERO` si l'entité est au centre (rayon nul → tangente indéfinie).
#[derive(Clone)]
pub struct RotateAround {
    center: Vec2,
    freq: f32,
}
impl RotateAround {
    pub fn new(center: Vec2, freq: f32) -> Self {
        Self { center, freq }
    }
}
impl Movement for RotateAround {
    fn evaluate(
        &mut self,
        at: Duration,
        deltatime: Duration,
        current_position: Vec2,
        velocity: Vec2,
        player_pos: Vec2,
    ) -> Vec2 {
        let local = current_position - self.center;
        let r = local.length();
        if r == 0.0 {
            return Vec2::ZERO;
        }
        let tangent = Vec2::new(-local.y, local.x) / r;
        tangent * 2.0 * std::f32::consts::PI * self.freq * r * deltatime.as_secs_f32()
    }
    fn clone_box(&self) -> Box<dyn Movement + Send + Sync> {
        Box::new(self.clone())
    }
}