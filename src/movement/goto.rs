use bevy::prelude::*;
use crate::movement::Movement;
use std::time::Duration;
#[derive(Clone)]
pub struct Goto {
    target:Vec2,
    speed:f32
}
impl Goto {
    pub fn new(

        target:Vec2,
        speed:f32
    )-> Self {
        Self {target,speed}
    }
}
impl Movement for Goto {
    fn evaluate(
        &mut self,
        at: Duration,
        deltatime: Duration,
        current_position: Vec2,
        velocity: Vec2,
        player_pos: Vec2,
    ) -> Vec2 {
        let delta = self.target - current_position;
        if let Some(direction) = delta.try_normalize() {
            // Évite l'overshoot : si on est à moins d'un pas de la cible, on
            // y va exactement (sinon on oscillerait autour à vitesse constante).
            let step = self.speed * deltatime.as_secs_f32();
            if step >= delta.length() {
                delta
            } else {
                direction * step
            }
        } else {
            Vec2::ZERO
        }
    }
    fn clone_box(&self) -> Box<dyn Movement + Send + Sync> {
        Box::new(self.clone())
    }
}