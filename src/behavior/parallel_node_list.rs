use crate::behavior::node_list::*;
use bevy::{ecs::system::EntityCommands};
use std::time::Duration;
use crate::behavior::behavior::Behavior; 
use crate::behavior::node::TimedNode;
use crate::behavior::node::UpdateSchedule;
pub struct ParallelNodeList {
    pub node_list:NodeList
}
impl ParallelNodeList {
    pub fn new()->Self {
        Self {node_list:NodeList::new()}
    }
    pub fn with(mut self,behavior:impl Behavior+ 'static)->Self {
        self.node_list.nodes.push(
            (TimedNode {behavior:Box::new(behavior),schedule:UpdateSchedule::Never},0.0)
        );
        self
    }
    pub fn update(&mut self,timedelta:Duration,mut cmd:EntityCommands) {
        for node in self.node_list.nodes.iter_mut() {
            node.0.behavior.update(timedelta, cmd.reborrow());
        }
    }
    pub fn enable(&mut self,mut ec: EntityCommands) {
        for node in self.node_list.nodes.iter_mut() {
            node.0.behavior.enable( ec.reborrow());
        }
    }
    pub fn disable(&mut self,mut ec: EntityCommands) {
        for node in self.node_list.nodes.iter_mut() {
            node.0.behavior.disable( ec.reborrow());
        }
    }
}