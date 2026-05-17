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
        at:Duration,
        deltatime:Duration,
        current_position:Vec2,
        velocity:Vec2,
        player_pos:Vec2
    )->Vec2 {  
        let delta = (self.target-current_position);
        if let Some(direction) = delta.try_normalize() {
            direction * deltatime.as_secs_f32() * self.speed /  (delta.x * delta.x + delta.y * delta.y)
        }
        else {
            Vec2::ZERO
        }
    }
    fn clone_box(&self) -> Box<dyn Movement + Send + Sync> {
        Box::new(self.clone())
    }
}