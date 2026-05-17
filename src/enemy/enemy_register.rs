use crate::{enemy::enemy_builder::EnemyBuilder, game_manager::difficulty::Difficulty};
use bevy::{prelude::*, time::Stopwatch, utils::HashMap};

#[derive(Resource)]
pub struct EnemyRegister(
    pub Vec<Box<dyn EnemyBuilder + Sync + Send + 'static>>
);
impl EnemyRegister {
    pub fn new() -> Self {Self(Vec::new())}
    pub fn with(mut self,enemy_builder:impl EnemyBuilder+ Sync + Send + 'static)->Self {
        self.0.push(Box::new(enemy_builder));
        self
    }
}


pub fn spawn(
    mut commands: Commands,
    mut difficulty: ResMut<Difficulty>,
    time: Res<Time>,
    windows: Query<&Window>,
    mut enemy_register: ResMut<EnemyRegister>,
    asset_server: Res<AssetServer>
) {
    for enemy_builder in enemy_register.0.iter_mut() {
        enemy_builder.spawns(
            commands.reborrow(), 
            windows.single(), 
            &time,
            &mut difficulty, 
            &asset_server
        );
    }   
}