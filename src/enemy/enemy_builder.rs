use bevy::prelude::*;
use bevy::render::view::window;
use bevy::utils::HashMap;
use crate::game_manager::difficulty::{Difficulty, SpawnPosition};
use crate::enemy::anim_bank;


pub trait EnemyBuilder {
    fn preload_anim(&self)->HashMap<&str, &str>;
    fn get_timer(&mut self)->&mut Timer;
    fn spawn(
        &self,
        commands: Commands,
        window: &Window,
        difficulty: &ResMut<Difficulty>,
        spawn_pos: SpawnPosition,
        asset_server:&Res<AssetServer>);
    fn name(&self)->&str;
    fn spawns(
        &mut self,
        mut commands: Commands,
        window: &Window,
        time: &Res<Time>,
        difficulty:&mut ResMut< Difficulty>,
        asset_server:&Res<AssetServer>
    ) {
        if let Some(&(wave_size, target_interval, spawn_pos)) = difficulty.active_spawners.get(self.name())
        {
            if (self.get_timer().duration().as_secs_f32() - target_interval).abs() > 0.01 {
                self.get_timer().set_duration(std::time::Duration::from_secs_f32(target_interval));
            }

            self.get_timer().tick(time.delta());
            if !self.get_timer().just_finished() {
                return;
            }
            for _ in 0..wave_size {
                self.spawn(commands.reborrow(),window, &difficulty, spawn_pos,&asset_server);
            }
            self.get_timer().reset();
        }
        let Some(pos) = difficulty
            .spawn_requests
            .iter()
            .position(|(n, _, _)| *n == self.name())
        else {
            return;
        };
        let (_name, count, spawn_pos) = difficulty.spawn_requests.remove(pos);

        for _ in 0..count {
            self.spawn(commands.reborrow(),window, &difficulty, spawn_pos,&asset_server);
            //spawn_one(&mut commands, &frames, window, spawn_pos);
        }
    }
}

