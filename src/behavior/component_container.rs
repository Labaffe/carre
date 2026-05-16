use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use std::time::Duration;
use std::sync::Arc;
use crate::behavior::behavior::Behavior;
pub struct ComponentContainer {
    // Arc so the closure is Clone-able (needed to call multiple times)
    pub inserter: Arc<dyn Fn(&mut EntityCommands) + Send + Sync>,
    pub remover:  Arc<dyn Fn(&mut EntityCommands) + Send + Sync>,
    active: bool,
}

impl ComponentContainer {
    pub fn new<C: Component + Clone>(component: C) -> Self {
        let insert_comp = component.clone();

        Self {
            inserter: Arc::new(move |ec: &mut EntityCommands| {
                ec.insert(insert_comp.clone());
            }),
            remover: Arc::new(|ec: &mut EntityCommands| {
                ec.remove::<C>();
            }),
            active: false,
        }
    }
}
impl Behavior for ComponentContainer {
    fn enable(&mut self, mut ec: EntityCommands) {
        if !self.active {
            (self.inserter)(&mut ec);
            self.active = true;
        }
    }

    fn disable(&mut self, mut ec: EntityCommands) {
        if self.active {
            (self.remover)(&mut ec);
            self.active = false;
        }
    }
    fn update(&mut self,_timedelta:Duration,_cmd:EntityCommands,transition_messages:&Vec<String>) {}
}
