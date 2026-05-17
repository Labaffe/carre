use bevy::prelude::*;
use std::time::Duration;
use bevy::time::Stopwatch;
use crate::movement::Movement;
use std::sync::{Arc, Mutex};

#[derive(Component)]
pub struct Movements {
    directions: Vec<Box<dyn Movement + Send + Sync>>,
    velocity:Vec2,
    time: Stopwatch,
    lag: Duration,
}
impl Clone for Movements {
    fn clone(&self) -> Self {
        Movements {
            directions: self.directions.iter().map(|m| m.clone_box()).collect(),
            velocity:Vec2::ZERO,
            time: self.time.clone(),
            lag: self.lag,
        }
    }
}
impl Movements {
    pub fn new() -> Self {
        Movements {
            directions: vec![],
            time: Stopwatch::new(),
            velocity: Vec2::ZERO,
            lag: Duration::ZERO,
        }
    }

    pub fn with(mut self, movement: impl Movement + Send + Sync + 'static) -> Self {
        self.directions.push(Box::new(movement));
        self
    }

    pub fn evaluate(
        &mut self, 
        deltatime: Duration, 
        current_position: Vec2, 
        player_pos: Vec2
    ) -> Vec2 {
        if self.time.elapsed() > self.lag {
            let real_time = self.time.elapsed() - self.lag;
            let velocity = self.directions
                .iter_mut()
                .map(|m| m.evaluate(
                    real_time, 
                    deltatime, 
                    current_position, 
                    self.velocity,
                    player_pos)
                )
                .sum();
            self.velocity = velocity;
            velocity
        } else {
            Vec2::ZERO
        }
    }

    pub fn lag(mut self, value: Duration) -> Self {
        self.lag = value;
        self
    }

    pub fn get_movement(&mut self, current_position: Vec2, deltatime: Duration, player_pos: Vec2) -> Vec2 {
        let delta_pos = self.evaluate(deltatime, current_position, player_pos);
        self.time.tick(deltatime);
        delta_pos
    }
}