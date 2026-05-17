use crate::behavior::{node::*,node_list::*};
use bevy::{ecs::system::EntityCommands, log::tracing::Instrument};
use std::{ops::Index, time::Duration};

pub struct IndexedNodeList {
    pub node_list:NodeList,
    pub current_index:Option<usize>,
    pub target_index:Option<usize>
}
impl IndexedNodeList {
    pub fn new()->Self {
        Self {node_list:NodeList::new(),current_index:None,target_index:None}
    }
    pub fn node(& self,i:usize)->& TimedNode {self.node_list.node(i)}
    pub fn node_mut(&mut self,i:usize)->&mut TimedNode {self.node_list.node_mut(i)}
    pub fn current_node_mut(&mut self)->&mut TimedNode {
        match self.current_index {
            Some(index)=> self.node_list.node_mut(index),
            None=>self.node_list.node_mut(0),
        }
    }
    pub fn change_state(&mut self,mut cmd:EntityCommands) {
        if self.current_index == self.target_index {return}
        if let Some(i) = self.current_index {
            self.node_mut(i).behavior.disable( cmd.reborrow());
        }
        if let Some(i) = self.target_index {
            self.node_mut(i).behavior.enable( cmd);
        }
        
        self.current_index = self.target_index;
        self.node_list.reset_time();
    }
    pub fn is_node_finished(& self)->bool {
        if let Some(index) = self.current_index {
            self.node_list.is_node_finished(self.node(index))
        }
        else {
            false
        }
    }
    
    pub fn tick(&mut self,time_delta:Duration) {
        self.node_list.tick(time_delta);
    }
}
use bevy::prelude::*;
pub trait NodeListDriver {
    fn get_indexed_node_list(&self)-> &IndexedNodeList; 
    fn get_indexed_node_list_mut(&mut self)-> &mut IndexedNodeList; 
    fn pick_next(&mut self);
    fn update(&mut self,timedelta:Duration,mut cmd:EntityCommands,transition_messages:&Vec<String>) {
        let my_span = info_span!("update ordered list", name = "update ordered list").entered();

        if self.enabled() {
            self.get_indexed_node_list_mut().current_node_mut().behavior.update(timedelta, cmd.reborrow(),transition_messages);
            self.get_indexed_node_list_mut().tick(timedelta);
            if self.get_indexed_node_list_mut().is_node_finished() {
                self.pick_next();
                self.get_indexed_node_list_mut().change_state(cmd);
            }
        }
    }
    fn enable(&mut self, ec: EntityCommands) {
        if !self.enabled() {
            self.reset_pick();
            self.get_indexed_node_list_mut().node_list.reset_time();
            self.get_indexed_node_list_mut().change_state(ec);
        } 
    }
    fn disable(&mut self, ec: EntityCommands) {
        if self.enabled() {
            self.get_indexed_node_list_mut().target_index = None;
            self.get_indexed_node_list_mut().change_state(ec);
        }
    }
    fn enabled(&self)-> bool {
        match self.get_indexed_node_list().target_index {
            Some(_i)=>true,
            None=>false
        }
    }
    fn reset_pick(&mut self);
}
