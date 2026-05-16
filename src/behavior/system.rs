use bevy::{ecs::system::Query, prelude::*, time::Time, transform::commands};

use crate::behavior::{behavior::BehaviorComponent, choice_list::TransitionMessages};

pub fn init_behavior(
    mut commands:Commands,
    time:Res<Time>,
    mut query:Query<(Entity,&mut BehaviorComponent)>
) {
    for (entity,mut behavior_component) in query.iter_mut() {
        behavior_component.timer.tick(time.delta());
        behavior_component.behavior.enable(commands.entity(entity));
    }
}


pub fn update_behavior(
    mut command:Commands,
    time:Res<Time>,
    mut query:Query<(Entity,&mut BehaviorComponent,&mut TransitionMessages)>
) {
    for (entity,mut behavior_component,mut transition_messages) in query.iter_mut() {
        if transition_messages.messages.len() >0 {
            println!("{:?}",transition_messages);
        }
        
        behavior_component.behavior.update(
            time.delta(),
            command.entity(entity),
            &transition_messages.messages
        );
        transition_messages.clear()
    }
}
