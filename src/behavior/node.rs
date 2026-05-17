use bevy::prelude::*;
use std::time::Duration;
use crate::behavior::behavior::Behavior;
use crate::behavior::component_container::ComponentContainer;

// A node knows how to activate and deactivate itself on an entity.
// Built at spawn time, driven at runtime by SequenceDriver.

pub struct Node {
    pub duration: f32,
    pub behavior:Box<dyn Behavior>,
}



impl Node {
    pub fn new<C>(duration: f32, component:C) -> Self
    where
        C: Component +Clone+ Send + Sync + 'static,
    {
        Self { duration,behavior:Box::new( ComponentContainer::new(component))  }
    }
}
pub enum UpdateSchedule {
    ByDuration(Duration),
    ByRate(f32),
    Never
}
impl UpdateSchedule {
    pub fn is_finished(self,elapsed:Duration,time_delta:Duration)-> bool {
        match self {
            Self::ByDuration(d)=> {d<elapsed},
            Self::ByRate(rate)=> {
                let dt = time_delta.as_secs_f32();
                let probability = 1.0 - (-rate * dt).exp();
                fastrand::f32() < probability
            },
            Self::Never=> false
        }
    }
}
pub struct TimedNode {
    pub behavior:Box<dyn Behavior>,
    pub schedule:UpdateSchedule
}
impl TimedNode {
    pub fn is_finished(&self,elapsed:Duration,time_delta:Duration)-> bool {
        match self.schedule {
            UpdateSchedule::ByDuration(d)=> {d<elapsed},
            UpdateSchedule::ByRate(rate)=> {
                let dt = time_delta.as_secs_f32();
                let probability = 1.0 - (-rate * dt).exp();
                fastrand::f32() < probability
            },
            UpdateSchedule::Never=> false
        }
    }
}
