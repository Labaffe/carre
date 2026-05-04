use bevy::prelude::*;
use bevy::utils::Duration;
pub trait Movement {
    fn evaluate(&self,at:Duration,delta:Duration,current_position:Vec2)->Vec2;
}