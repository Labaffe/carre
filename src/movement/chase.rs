use bevy::prelude::*;
use crate::movement::Movement;
use bevy::utils::Duration;
#[derive(Clone)]
pub struct Chase {
    speed:f32
}
impl Chase {
    pub fn new(
        speed:f32
    )-> Self {
        Chase {speed}
    }
}
impl Movement for Chase {
    fn evaluate(&mut self,at:Duration,delta:Duration,current_position:Vec2,player_position:Vec2)->Vec2 {
        let direction = (player_position-current_position).normalize_or_zero();
        direction * self.speed
    }
    fn clone_box(&self) -> Box<dyn Movement + Send + Sync> {
        Box::new(self.clone())
    }
}