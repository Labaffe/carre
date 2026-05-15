use bevy::prelude::*;
use crate::movement::Movement;
use bevy::utils::Duration;
#[derive(Clone)]
pub struct Rush {
    speed:f32,
    target_pos:Vec2,
    already_set:bool
}
impl Rush {
    pub fn new(
        speed:f32
    )-> Self {
        Rush {speed,target_pos:Vec2::ZERO,already_set:false}
    }
}
impl Movement for Rush {
    fn evaluate(&mut self,at:Duration,delta:Duration,current_position:Vec2,player_position:Vec2)->Vec2 {
        if !self.already_set {
            self.target_pos = player_position;
            self.already_set=true;
        }
        
        let direction = (self.target_pos-current_position).normalize_or_zero();
        direction * self.speed
    }
    fn clone_box(&self) -> Box<dyn Movement + Send + Sync> {
        Box::new(self.clone())
    }
}