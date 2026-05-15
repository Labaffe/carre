use bevy::prelude::*;
use crate::movement::Movement;
use bevy::utils::Duration;
#[derive(Clone)]
pub struct Sinusoid {
    amplitude:f32,
    pulse:f32,
    phase:f32,
    direction:Vec2
}
impl Sinusoid {
    pub fn new(
        amplitude:f32,
        pulse:f32,
        phase:f32,
        direction:Vec2
    )-> Self {
        Sinusoid {amplitude,pulse,phase,direction}
    }
}
impl Movement for Sinusoid {
    fn evaluate(&mut self,at:Duration,delta:Duration,current_position:Vec2,player_pos:Vec2)->Vec2 {
        self.amplitude * (
            (self.pulse * (at+delta).as_secs_f32() + self.phase).sin() 
            - (self.pulse * (at).as_secs_f32() + self.phase).sin()
        ) * self.direction
    }
    fn clone_box(&self) -> Box<dyn Movement + Send + Sync> {
        Box::new(self.clone())
    }
}