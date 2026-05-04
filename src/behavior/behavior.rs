use crate::behavior::parallel_node_list::{ ParallelNodeList};
use crate::behavior::weighted_list:: WeightedNodeList;
use crate::behavior::ordered_list:: OrderedNodeList;

use crate::behavior::indexed_node_list:: NodeListDriver;
use crate::behavior::component_container::ComponentContainer;
use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use std::time::Duration;

/* 
pub enum Behavior {

    Comp(ComponentContainer),
    WeightedList(WeightedNodeList),
    OrderedList(OrderedNodeList),
    ParallelList(ParallelNodeList)
}
impl Behavior {
    pub fn from_component<C: Component + Clone>(component: C)->Self {
        Behavior::Comp(ComponentContainer::new(component))
    }
    pub fn enable(&mut self, ec: EntityCommands) {
        match self {
            Behavior::Comp(componentcontainer)=> {
                componentcontainer.enable(ec);
            }
            Behavior::WeightedList(weighted_list)=> {
                weighted_list.enable(ec);
            }
            Behavior::OrderedList(ordered_list)=> {
                ordered_list.enable(ec);
            }
            Behavior::ParallelList(parallel_node_list)=> {
                parallel_node_list.enable(ec);
            }
        }
    }

    pub fn disable(&mut self, ec: EntityCommands) {
        match self {
            Behavior::Comp(componentcontainer)=> {
                componentcontainer.disable(ec);
            }
            Behavior::WeightedList(weighted_list)=> {
                weighted_list.disable(ec);
            }
            Behavior::OrderedList(ordered_list)=> {
                ordered_list.disable(ec);
            }
            Behavior::ParallelList(parallel_node_list)=> {
                parallel_node_list.disable(ec);
            }
        }
    }
    pub fn update(&mut self,timedelta:Duration,ec:EntityCommands) {
        let my_span = info_span!("update behavior (match)", name = "update behavior (match)").entered();
        match self {
            Behavior::Comp(componentcontainer)=> {
                componentcontainer.update(timedelta,ec);
            }
            Behavior::WeightedList(weighted_list)=> {
                weighted_list.update(timedelta,ec);
            }
            Behavior::OrderedList(ordered_list)=> {
                ordered_list.update(timedelta,ec);
            }
            Behavior::ParallelList(parallel_node_list)=> {
                parallel_node_list.update(timedelta,ec);
            }
        }
    }
}
*/
pub trait Behavior:Send + Sync + 'static{
    fn enable(&mut self, ec: EntityCommands);
    fn disable(&mut self, ec: EntityCommands);
    fn update(&mut self,timedelta:Duration,ec:EntityCommands);
}
#[derive(Component)]
pub struct BehaviorComponent {
    pub behavior:Box<dyn Behavior + Send + Sync>,
    pub timer:Timer
}
impl BehaviorComponent {
    pub fn new(behavior:impl Behavior + Send + Sync + 'static)->Self {
        Self {behavior:Box::new(behavior),timer:Timer::new(Duration::ZERO, TimerMode::Once)}
    }
}