use bevy::prelude::*;
use crate::movement::Movement;
use bevy::utils::Duration;
#[derive(Clone)]
pub struct Translate {
    direction:Vec2,
    speed:f32
}
impl Translate {
    pub fn new(
        direction:Vec2,
        speed:f32
    )-> Self {
        Self {direction,speed}
    }
}
impl Movement for Translate {
    fn evaluate(&self,at:Duration,deltatime:Duration,current_position:Vec2)->Vec2 {  
        self.direction * deltatime.as_secs_f32() * self.speed
    }
}