use std::time::Duration;

use crate::{behavior::{ indexed_node_list::*,behavior::Behavior}};
use crate::behavior::node::{TimedNode,UpdateSchedule};
pub struct OrderedNodeList {
    pub indexed_node_list:IndexedNodeList,
    pub looping: bool
}
impl NodeListDriver for OrderedNodeList {
    fn pick_next(&mut self) {
        if let Some(i) = self.indexed_node_list.current_index {
            let next = i + 1;
            if next >= self.indexed_node_list.node_list.nodes.len() {
                if self.looping {
                    self.indexed_node_list.target_index = Some(0);
                } else {
                    self.indexed_node_list.target_index = None;
                }
            } else {
                self.indexed_node_list.target_index = Some(next);
            }
        }
    }
    fn get_indexed_node_list(&self)->&IndexedNodeList {&self.indexed_node_list}
    fn get_indexed_node_list_mut(&mut self)->&mut IndexedNodeList {&mut self.indexed_node_list}
    fn reset_pick(&mut self) {
        self.indexed_node_list.target_index = Some(0);
    }
}

impl OrderedNodeList {
    pub fn new()->Self {Self {indexed_node_list:IndexedNodeList::new(),looping:false  }}
    pub fn then(mut self,duration:Duration,behavior:impl Behavior+ 'static)->Self {
        self.indexed_node_list.node_list.nodes.push(
            (TimedNode {behavior:Box::new(behavior),schedule:UpdateSchedule::ByDuration(duration)},0.0)
        );
        self
    }
    pub fn should_loop(mut self)->Self {
        self.looping = true;
        self
    }
}
use bevy::ecs::system::EntityCommands;
impl Behavior for OrderedNodeList {
    fn enable(&mut self,ec: EntityCommands<'_>) {NodeListDriver::enable(self, ec);}
    fn disable(&mut self,ec: EntityCommands<'_>) {NodeListDriver::disable(self, ec);}
    fn update(&mut self,timedelta:Duration,ec: EntityCommands<'_>,transition_messages:&Vec<String>) {NodeListDriver::update(self,timedelta,ec,transition_messages);}
}