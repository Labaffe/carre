use std::time::Duration;

use crate::{behavior::{ indexed_node_list::*,behavior::Behavior}};
use crate::behavior::node::{TimedNode,UpdateSchedule};
pub struct ChoiceNodeList {
    pub indexed_node_list:IndexedNodeList,
}
impl NodeListDriver for ChoiceNodeList {
    fn pick_next(&mut self) { }
    fn get_indexed_node_list(&self)->&IndexedNodeList {&self.indexed_node_list}
    fn get_indexed_node_list_mut(&mut self)->&mut IndexedNodeList {&mut self.indexed_node_list}
    fn reset_pick(&mut self) {
        self.indexed_node_list.target_index = Some(0);
    }
}

impl ChoiceNodeList {
    pub fn new()->Self {Self {indexed_node_list:IndexedNodeList::new() }}
    pub fn with(mut self,behavior:impl Behavior+ 'static)->Self {
        self.indexed_node_list.node_list.nodes.push(
            (TimedNode {behavior:Box::new(behavior),schedule:UpdateSchedule::Never},0.0)
        );
        self
    }
    pub fn should_loop(mut self)->Self {
        self.looping = true;
        self
    }
}
use bevy::ecs::system::EntityCommands;
impl Behavior for ChoiceNodeList {
    fn enable(&mut self,ec: EntityCommands<'_>) {NodeListDriver::enable(self, ec);}
    fn disable(&mut self,ec: EntityCommands<'_>) {NodeListDriver::disable(self, ec);}
    fn update(&mut self,timedelta:Duration,ec: EntityCommands<'_>) {NodeListDriver::update(self,timedelta,ec);}
}