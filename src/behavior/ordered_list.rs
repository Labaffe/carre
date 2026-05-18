use std::time::Duration;

use crate::behavior::choice_list::TransitionMessages;
use crate::behavior::{behavior::Behavior, indexed_node_list::*};
use crate::behavior::node::{TimedNode, UpdateSchedule};
use bevy::ecs::system::EntityCommands;

pub struct OrderedNodeList {
    pub indexed_node_list: IndexedNodeList,
    pub looping: bool,
    /// Si présent, ce message est poussé dans `TransitionMessages` quand la
    /// liste termine son dernier node (et `looping = false`). Permet à un
    /// parent `ChoiceNodeList` de transitionner sur la fin d'une sub-behavior
    /// temporisée (ex: fin d'une phase de transition boss).
    pub completion_message: Option<&'static str>,
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
    pub fn new()->Self {Self {indexed_node_list:IndexedNodeList::new(),looping:false,completion_message:None }}
    pub fn then(mut self,duration:Duration,behavior:impl Behavior+ 'static)->Self {
        self.indexed_node_list.node_list.nodes.push(
            (TimedNode {behavior:Box::new(behavior),schedule:UpdateSchedule::ByDuration(duration)},0.0)
        );
        self
    }
    pub fn during(mut self, duration:Duration)->Self {
        if let Some(last) = self.indexed_node_list.node_list.nodes.last_mut() {
            last.0.schedule = UpdateSchedule::ByDuration(duration);
        }
        self
    }
    pub fn should_loop(mut self)->Self {
        self.looping = true;
        self
    }
    /// Définit un message à pousser dans `TransitionMessages` quand la liste
    /// termine (dernier node fini, hors mode `should_loop`). Le push passe par
    /// `EntityCommands::entry::<TransitionMessages>().and_modify(...)` donc le
    /// message apparaît une frame plus tard côté consommateur.
    pub fn on_complete(mut self, message: &'static str) -> Self {
        self.completion_message = Some(message);
        self
    }
}
impl Behavior for OrderedNodeList {
    fn enable(&mut self, ec: EntityCommands<'_>) { NodeListDriver::enable(self, ec); }
    fn disable(&mut self, ec: EntityCommands<'_>) { NodeListDriver::disable(self, ec); }
    fn update(&mut self, timedelta: Duration, mut ec: EntityCommands<'_>, transition_messages: &Vec<String>) {
        let was_enabled = self.enabled();
        NodeListDriver::update(self, timedelta, ec.reborrow(), transition_messages);
        // Détecte la transition "enabled → completed" (target_index passé à None
        // par `pick_next` quand on dépasse le dernier node sans loop).
        if was_enabled && !self.enabled() {
            if let Some(msg) = self.completion_message {
                let msg_owned = msg.to_string();
                ec.entry::<TransitionMessages>().and_modify(move |mut m| {
                    m.messages.push(msg_owned);
                });
            }
        }
    }
}
