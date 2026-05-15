
pub mod node;
pub mod node_list;
pub mod weighted_list;
pub mod ordered_list;
pub mod behavior;
pub mod component_container;
pub mod indexed_node_list;
mod system;
mod choice_list;
pub mod parallel_node_list;

use bevy::prelude::*;
use crate::behavior::{behavior::Behavior, choice_list::ChoiceNodeList, component_container::ComponentContainer, ordered_list::OrderedNodeList, parallel_node_list::ParallelNodeList, system::{init_behavior, update_behavior}};
use crate::behavior::indexed_node_list::NodeListDriver;
use bevy::ecs::system::EntityCommands;
use std::time::Duration;
pub struct BehaviorPlugin;

impl Plugin for BehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (update_behavior,init_behavior));
    }
}

pub struct Empty;
impl Behavior for Empty {
    fn enable(&mut self, ec: EntityCommands) {}
    fn disable(&mut self, ec: EntityCommands) {}
    fn update(&mut self,timedelta:Duration,ec:EntityCommands) {}
}

pub struct BehaviorBuilder {}
impl BehaviorBuilder {
    pub fn choice()->ChoiceNodeList {
        ChoiceNodeList::new()
    }
    pub fn first(duration: Duration,behavior:impl Behavior)->OrderedNodeList {
        OrderedNodeList::new().then(duration,behavior)
    }
    pub fn nothing()->Empty {Empty}
    pub fn from_component<C: Component + Clone>(component: C) ->ComponentContainer {
        ComponentContainer::new(component)
    }
    pub fn multiple()->ParallelNodeList {
        ParallelNodeList::new()
    }
}
