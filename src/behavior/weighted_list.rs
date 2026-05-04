use crate::behavior::indexed_node_list::*;


pub struct WeightedNodeList {
    pub indexed_node_list:IndexedNodeList
}
impl NodeListDriver for WeightedNodeList {
    fn pick_next(&mut self) {
        let total: f32 = self.indexed_node_list.node_list.nodes.iter().map(|node| node.1.max(0.0)).sum();
        if total <= 0.0 {
            self.indexed_node_list.target_index = None;
        }
        let mut pick = fastrand::f32() * total;
        for (i,node) in self.indexed_node_list.node_list.nodes.iter().enumerate() {
            pick -= node.1.max(0.0);
            if pick <= 0.0 {
                self.indexed_node_list.target_index = Some(i);
            }
        }
        self.indexed_node_list.target_index =  None;
    }
    fn get_indexed_node_list_mut(&mut self)-> &mut IndexedNodeList {
        &mut self.indexed_node_list
    }
    fn get_indexed_node_list(&self)->&IndexedNodeList {&self.indexed_node_list}

    fn reset_pick(&mut self) {
        self.pick_next();
    }
}

