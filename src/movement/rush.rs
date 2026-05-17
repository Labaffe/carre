use bevy::prelude::*;
use crate::movement::Movement;
use std::time::Duration;
#[derive(Clone)]
pub struct Rush {
    speed:f32,
    already_set:bool,
    direction:Vec2
}
impl Rush {
    pub fn new(
        speed:f32
    )-> Self {
        Rush {speed,already_set:false,direction:Vec2::ZERO}
    }
}
impl Movement for Rush {
    fn evaluate(
        &mut self,
        at:Duration,
        deltatime:Duration,
        current_position:Vec2,
        velocity:Vec2,
        player_pos:Vec2
    )->Vec2 { 
        if !self.already_set {
            self.direction = (player_pos-current_position).normalize_or_zero();
            self.already_set=true;
        }
        self.direction * self.speed * deltatime.as_secs_f32()
    }
    fn clone_box(&self) -> Box<dyn Movement + Send + Sync> {
        Box::new(self.clone())
    }
}