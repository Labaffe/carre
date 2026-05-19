use bevy::prelude::*;
use crate::movement::Movement;
use std::time::Duration;
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
    fn evaluate(
        &mut self,
        at: Duration,
        deltatime: Duration,
        current_position: Vec2,
        velocity: Vec2,
        player_pos: Vec2,
    ) -> Vec2 {
        let direction = (player_pos - current_position).normalize_or_zero();
        direction * self.speed * deltatime.as_secs_f32()
    }
    fn clone_box(&self) -> Box<dyn Movement + Send + Sync> {
        Box::new(self.clone())
    }
}