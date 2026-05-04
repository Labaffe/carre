use bevy::prelude::*;
use bevy::utils::Duration;
use bevy::time::Stopwatch;
use crate::movement::Movement;
use std::sync::Arc;
#[derive(Component,Clone)]
pub struct Movements {
    directions:Vec<Arc<dyn Movement + Send + Sync>>,
    time:Stopwatch,
    lag:Duration
}
impl Movements {
    pub fn new()->Self {
        Movements {directions:vec![],time:Stopwatch::new(),lag:Duration::ZERO}
    }
    pub fn with(mut self,movement:impl Movement+Send+Sync+'static)->Self {
        self.directions.push(Arc::new(movement));
        self
    }
    pub fn evaluate(&self,deltatime:Duration,current_position:Vec2)-> Vec2 {
        if self.time.elapsed()>self.lag {
            let real_time = self.time.elapsed()-self.lag;
            self.directions.iter()
            .map(|m| m.evaluate(real_time,deltatime,current_position))
            .sum()
        }
        else {
            Vec2::ZERO
        }
    }
    pub fn lag(mut self,value:Duration)->Self {
        self.lag = value;
        self
    }
    pub fn get_movement(&mut self,current_position:Vec2,deltatime:Duration)->Vec2 {
        let delta_pos = self.evaluate(deltatime,current_position);
        self.time.tick(deltatime);
        delta_pos
    }
}