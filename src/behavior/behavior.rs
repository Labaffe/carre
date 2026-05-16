use crate::behavior::choice_list::{ChoiceNodeList, TransitionMessages};
use crate::behavior::parallel_node_list::{ ParallelNodeList};
use crate::behavior::weighted_list:: WeightedNodeList;
use crate::behavior::ordered_list:: OrderedNodeList;

use crate::behavior::indexed_node_list:: NodeListDriver;
use crate::behavior::component_container::ComponentContainer;
use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use bevy::utils::HashMap;
use std::time::Duration;

pub trait Behavior:Send + Sync + 'static{
    fn enable(&mut self, ec: EntityCommands);
    fn disable(&mut self, ec: EntityCommands);
    fn update(&mut self,timedelta:Duration,ec:EntityCommands,transitions:&Vec<String>);
}
#[derive(Component)]
pub struct BehaviorComponent {
    pub behavior:Box<dyn Behavior + Send + Sync>,
    pub timer:Timer
}
impl BehaviorComponent {
    pub fn new(behavior:impl Behavior + Send + Sync + 'static)->Self {
        Self {
            behavior:Box::new(behavior),
            timer:Timer::new(Duration::ZERO, TimerMode::Once)
        }
    }
}