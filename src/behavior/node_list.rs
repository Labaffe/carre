use crate::behavior::node::*;
use bevy::time::Stopwatch;
use std::time::Duration;

pub struct NodeList {
    pub nodes:Vec<(TimedNode,f32)>,
    pub stopwatch:Option<Stopwatch>,
    pub time_delta:Duration
}
impl NodeList {
    pub fn new()->Self { Self{nodes:vec![],stopwatch:Some(Stopwatch::new()),time_delta:Duration::ZERO}}
    pub fn node(& self,i:usize)->& TimedNode {
        &self.nodes[i].0
    }
    pub fn node_mut(&mut self,i:usize)->&mut TimedNode {
        &mut self.nodes[i].0
    }
    pub fn reset_time(&mut self) {
        if let Some(stopwatch) =&mut self.stopwatch {
            stopwatch.reset();  
        }
    }
    pub fn tick(&mut self,time_delta:Duration) {
        if let Some(stopwatch) = &mut self.stopwatch {
            stopwatch.tick(time_delta);
            self.time_delta = time_delta;
        }
    }
    pub fn is_node_finished(&self,node:&TimedNode)->bool {
        match  &self.stopwatch {
            Some(stopwatch) => node.is_finished(
                Duration::from_secs_f32(stopwatch.elapsed_secs()), 
                self.time_delta
            ),
            None => false
        }
    }
}
