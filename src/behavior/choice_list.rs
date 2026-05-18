use std::time::Duration;

use crate::behavior::{behavior::Behavior, indexed_node_list::*};
use crate::behavior::node::{TimedNode, UpdateSchedule};
use bevy::ecs::{component::Component, system::EntityCommands};

pub struct ChoiceNodeList {
    pub indexed_node_list: IndexedNodeList,
    /// Transitions stockées dans l'ordre de déclaration : `(message, from, to)`.
    /// `consume_messages` itère cette liste et applique la PREMIÈRE transition
    /// dont le message est présent ET dont `from` matche `current_index`.
    /// L'ordre de déclaration définit donc la priorité quand plusieurs
    /// transitions peuvent tirer depuis le même état dans la même frame.
    transitions: Vec<(String, usize, usize)>,
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
    pub fn new() -> Self {
        Self {
            indexed_node_list: IndexedNodeList::new(),
            transitions: Vec::new(),
        }
    }
    pub fn with(mut self, behavior: impl Behavior + 'static) -> Self {
        self.indexed_node_list.node_list.nodes.push(
            (TimedNode { behavior: Box::new(behavior), schedule: UpdateSchedule::Never }, 0.0)
        );
        self
    }
    pub fn add_transition(mut self, from: usize, to: usize, message: &str) -> ChoiceNodeList {
        self.transitions.push((message.to_string(), from, to));
        self
    }
    fn consume_messages(&mut self, transition_messages: &Vec<String>, ec: EntityCommands<'_>) {
        let Some(current) = self.indexed_node_list.current_index else { return; };
        for (msg, from, to) in &self.transitions {
            if *from == current && transition_messages.iter().any(|m| m == msg) {
                self.indexed_node_list.target_index = Some(*to);
                self.indexed_node_list.change_state(ec);
                return;
            }
        }
    }
}
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