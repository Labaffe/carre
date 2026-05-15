use bevy::prelude::*;
use bevy::utils::Duration;
pub trait Movement {
    fn evaluate(&mut self,at:Duration,delta:Duration,current_position:Vec2,player_pos:Vec2)->Vec2;
    fn clone_box(&self) -> Box<dyn Movement + Send + Sync>;
}