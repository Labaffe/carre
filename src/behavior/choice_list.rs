use std::{ time::Duration};

use crate::{behavior::{ indexed_node_list::*,behavior::Behavior}};
use crate::behavior::node::{TimedNode,UpdateSchedule};
pub struct ChoiceNodeList {
    pub indexed_node_list:IndexedNodeList,
    transitions:HashMap<String,(usize,usize)>
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
    pub fn new()->Self {Self {indexed_node_list:IndexedNodeList::new(),transitions:HashMap::new() }}
    pub fn with(mut self,behavior:impl Behavior+ 'static)->Self {
        self.indexed_node_list.node_list.nodes.push(
            (TimedNode {behavior:Box::new(behavior),schedule:UpdateSchedule::Never},0.0)
        );
        self
    }
    pub fn add_transition(mut self,from:usize,to:usize,message:&str)->ChoiceNodeList {
        self.transitions.insert(message.to_string(), (from,to));
        self
    }
    fn consume_messages(&mut self,transition_messages:&Vec<String>,ec: EntityCommands<'_>) {

        for message in transition_messages.iter() {
            if let Some(pair) = self.transitions.get(&message.to_string()) {
                let (from, to) = pair;
                if let Some(index) = self.indexed_node_list.current_index {
                    if index == *from {
                        self.indexed_node_list.target_index = Some(*to);
                        self.indexed_node_list.change_state(ec);
                        return;
                    }
                }
            }
        }
    }
}
use bevy::{ecs::{component::Component, system::EntityCommands}, log::tracing_subscriber::field::debug, utils::HashMap};
impl Behavior for ChoiceNodeList {
    fn enable(&mut self,ec: EntityCommands<'_>) {NodeListDriver::enable(self, ec);}
    fn disable(&mut self,ec: EntityCommands<'_>) {NodeListDriver::disable(self, ec);}
    fn update(&mut self,timedelta:Duration,mut ec: EntityCommands<'_>,transition_messages:&Vec<String>) {
        self.consume_messages(transition_messages,ec.reborrow());
        NodeListDriver::update(self,timedelta,ec,transition_messages);
    }
    
}
#[derive(Component,Debug)]
pub struct TransitionMessages{
    pub messages:Vec<String>
}
impl TransitionMessages {
    pub fn new()->Self {Self{messages:Vec::new()}}
    pub fn clear(&mut self) {
        self.messages = Vec::new();
    }
}