use bevy::prelude::*;
use crate::movement::Movement;
use bevy::utils::Duration;
#[derive(Clone)]
pub struct RotateAround {
    center:Vec2,
    freq:f32
}
impl RotateAround {
    pub fn new(
        center:Vec2,
        freq:f32
    )-> Self {
        Self {center,freq}
    }
}
impl Movement for RotateAround {
    fn evaluate(
        &mut self,
        at:Duration,
        deltatime:Duration,
        current_position:Vec2,
        velocity:Vec2,
        player_pos:Vec2
    )->Vec2 { 
        let local_pos = (current_position-self.center).normalize_or_zero();
        let distance = local_pos.length();
        Vec2::new(-local_pos.y,local_pos.x) * 2.0 * std::f32::consts::PI * self.freq
    }
    fn clone_box(&self) -> Box<dyn Movement + Send + Sync> {
        Box::new(self.clone())
    }
}