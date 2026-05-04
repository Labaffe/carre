use crate::enemy::enemy_builder::EnemyBuilder;
use bevy::prelude::*;

#[derive(Resource)]
pub struct EnemyRegister(Vec<Box<dyn EnemyBuilder + Sync + Send + 'static>>,);
impl EnemyRegister {
    pub fn new() -> Self {Self(Vec::new())}
    pub fn with(mut self,enemy_builder:dyn EnemyBuilder+ Sync + Send + 'static)->Self {
        self.0.push(Box::new(enemy_builder));
        self
    }
}

fn init_register(commands:Commands) {
    commands.insert_resource(resource);
}