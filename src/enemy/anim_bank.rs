use bevy::{prelude::*, utils::hashbrown::HashMap};
use std::{ time::Duration};

use crate::enemy::enemy_register::EnemyRegister;

#[derive(Resource)]
pub struct AnimBank {
    frames:HashMap<String,Vec<Handle<Image>>>
}
impl AnimBank {
    pub fn new() -> Self {
        Self {frames:HashMap::new()}
    }
    pub fn get(&self,name:&String) -> Option<&Vec<Handle<Image>>> {
        self.frames.get(name)
    }
    pub fn load_frames_from_folder(
        &mut self,
        name:String,
        asset_server: &Res<AssetServer>,
        folder: &str,
    ) {
        let dir_path = std::path::Path::new("assets").join(folder);
        let read_dir = std::fs::read_dir(&dir_path).ok();

        let mut entries: Vec<(usize, String)> = Vec::new();
        if let Some(dir) = read_dir {
            for entry in dir.flatten() {
                
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
}

#[derive(Component,Clone)]
pub struct Animation {
    name:String,
    current_frame:usize,
    timer:Timer,
    init:bool
}
impl Animation {
    pub fn new(name:&str,duration:Duration) -> Animation {
        Self {name:name.to_string(),current_frame:0,timer:Timer::new(duration, TimerMode::Repeating),init:false}
    } 
}
pub fn preload_frames(
    asset_server: Res<AssetServer>,
    mut anim_bank:ResMut<AnimBank>,
    enemy_register:Res<EnemyRegister>
) {
    //let mut hash_map: HashMap<&str,&str> = HashMap::new();
    for enemy_builder in enemy_register.0.iter() {
        for (name,folder) in enemy_builder.preload_anim().iter() {
            anim_bank.load_frames_from_folder(String::from(*name), &asset_server, folder);
        }
    }
}

pub fn animate(
    time: Res<Time>,
    anim_bank:Res<AnimBank>,
    mut query: Query<(&mut Sprite, &mut Animation)>
) {
    for (mut sprite, mut anim) in query.iter_mut() {
        if !anim.init {
            let frames = anim_bank.get(&anim.name);
            if let Some(f) = frames {
                anim.current_frame = 0;
                sprite.image = f[anim.current_frame].clone();
            }
            anim.init = true;
        }
        anim.timer.tick(time.delta());
        if anim.timer.just_finished() {
            let frames = anim_bank.get(&anim.name);
            if let Some(f) = frames {
                anim.current_frame = (anim.current_frame + 1) % f.len();
                sprite.image = f[anim.current_frame].clone();
            }
        }
    }
}
