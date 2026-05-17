use bevy::prelude::*;
use crate::movement::Movement;
use std::time::Duration;
#[derive(Clone)]
pub struct Shake {
    amplitude:f32,
    duration:f32,
}
impl Shake {
    pub fn new(
        amplitude:f32,
        duration:f32
    )-> Self {
        Shake {amplitude,duration}
    }
}
impl Movement for Shake {
    fn evaluate(
        &mut self,
        at:Duration,
        deltatime:Duration,
        current_position:Vec2,
        velocity:Vec2,
        player_pos:Vec2
    )->Vec2 { 
        let progress = at.as_secs_f32() / self.duration;
        let shake = progress * progress * self.amplitude;
        let dx = (fastrand::f32() - 0.5) * 2.0 * shake;
        let dy = (fastrand::f32() - 0.5) * 2.0 * shake;
        Vec2::new(dx,dy)
    }
    fn clone_box(&self) -> Box<dyn Movement + Send + Sync> {
        Box::new(self.clone())
    }
}

