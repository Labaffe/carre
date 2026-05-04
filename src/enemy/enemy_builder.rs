use bevy::prelude::*;
use crate::enemy::spawn::SpawnPosition;
use crate::game_manager::difficulty::Difficulty;
use crate::enemy::anim_bank;

pub trait EnemyBuilder {
    fn preload_anim(&self,loader:impl Fn(String,&str));
    fn spawn(commands: Commands,window: &Window,difficulty: ResMut<Difficulty>,windows: Query<&Window>,spawn_pos: SpawnPosition);
}