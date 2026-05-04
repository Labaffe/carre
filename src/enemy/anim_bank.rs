use bevy::prelude::*;
use std::{collections::HashMap, time::Duration};

use crate::enemy::enemy_register::EnemyRegister;

#[derive(Resource)]
pub struct AnimBank {
    frames:HashMap<String,Vec<Handle<Image>>>
}
impl AnimBank {
    pub fn new() -> Self {
        Self {frames:HashMap::new()}
    }
    pub fn get(&self,name:String) -> Option<&Vec<Handle<Image>>> {
        self.frames.get(&name)
    }
    pub fn load_frames_from_folder(
        mut self,
        name:String,
        asset_server: &Res<AssetServer>,
        folder: &str,
    ) {
        let dir_path = std::path::Path::new("assets").join(folder);
        let read_dir = std::fs::read_dir(&dir_path).ok()?;

        let mut entries: Vec<(usize, String)> = Vec::new();

        for entry in read_dir.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy().to_string();
            // Cherche les fichiers frameNNN.png (peu importe le nombre de chiffres)
            if let Some(rest) = name.strip_prefix("frame") {
                if let Some(num_str) = rest.strip_suffix(".png") {
                    if let Ok(index) = num_str.parse::<usize>() {
                        entries.push((index, name));
                    }
                }
            }
        }

        if entries.is_empty() {
            return;
        }

        // Tri par index croissant pour jouer les frames dans l'ordre
        entries.sort_by_key(|(i, _)| *i);

        let frames = entries
            .into_iter()
            .map(|(_, name)| asset_server.load(format!("{}/{}", folder, name)))
            .collect();

        self.frames.insert(name, frames);
    }
}

#[derive(Component)]
pub struct Animation {
    name:String,
    current_frame:usize,
    timer:Timer
}
impl Animation {
    pub fn new(name:&str,duration:Duration) -> Animation {
        Self {name:name.to_string(),current_frame:0,timer:Timer::new(duration, TimerMode::Repeating)}
    } 
}
pub fn preload_frames(anim_bank:AnimBank,mut commands: Commands, asset_server: Res<AssetServer>,enemy_register:Res<EnemyRegister>) {
    for enemy_builder in enemy_register.iter() {
        enemy_builder.preload_anim(|name:String,folder:&str| anim_bank.load_frames_from_folder(name, &asset_server, folder))
    }
}
pub fn animate(
    time: Res<Time>,
    anim_bank:AnimBank,
    mut query: Query<(&mut Handle<Image>, &mut Animation)>
) {
    for (mut texture, mut anim) in query.iter_mut() {
        anim.timer.tick(time.delta());
        if anim.timer.just_finished() {
            let frames = anim_bank.get(anim.name);
            anim.current_frame = (anim.current_frame + 1) % frames.0.len();
            *texture = frames.0[anim.current_frame].clone();
        }
    }
}
