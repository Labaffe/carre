use bevy::{prelude::*, platform::collections::HashMap};
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

#[derive(Component, Clone)]
pub struct Animation {
    name: String,
    current_frame: usize,
    timer: Timer,
    init: bool,
    /// Si `true`, l'animation s'arrête sur la dernière frame au lieu de reboucler.
    one_shot: bool,
    completed: bool,
    /// Si présent, la durée par frame est recalculée à l'init en fonction du
    /// nombre de frames chargées : `per_frame = total_duration / frame_count`.
    /// Utile pour piloter la durée totale d'une animation (ex: mort) sans
    /// dépendre du nombre exact de frames.
    total_duration: Option<Duration>,
}
impl Animation {
    /// Constructeur par durée par frame.
    pub fn new(name: &str, duration: Duration) -> Animation {
        Self {
            name: name.to_string(),
            current_frame: 0,
            timer: Timer::new(duration, TimerMode::Repeating),
            init: false,
            one_shot: false,
            completed: false,
            total_duration: None,
        }
    }
    /// Constructeur par durée totale : la durée par frame est calculée
    /// automatiquement à l'initialisation, une fois les frames chargées.
    /// Indépendant du nombre de frames du dossier — ajoute/retire des frames
    /// sans toucher au code.
    pub fn with_total_duration(name: &str, total: Duration) -> Animation {
        Self {
            name: name.to_string(),
            current_frame: 0,
            // Timer placeholder, sera recalculé à l'init.
            timer: Timer::new(Duration::from_millis(100), TimerMode::Repeating),
            init: false,
            one_shot: false,
            completed: false,
            total_duration: Some(total),
        }
    }
    pub fn one_shot(mut self) -> Self {
        self.one_shot = true;
        self
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
    anim_bank: Res<AnimBank>,
    mut query: Query<(&mut Sprite, &mut Animation)>,
) {
    for (mut sprite, mut anim) in query.iter_mut() {
        let Some(f) = anim_bank.get(&anim.name) else { continue };
        if f.is_empty() { continue; }

        if !anim.init {
            anim.current_frame = 0;
            sprite.image = f[0].clone();
            // Si une durée totale a été fournie, recalcule la durée par
            // frame maintenant que le nombre de frames est connu.
            if let Some(total) = anim.total_duration {
                let per_frame = total / (f.len() as u32);
                anim.timer = Timer::new(per_frame, TimerMode::Repeating);
            }
            anim.init = true;
        }

        if anim.completed { continue; }

        anim.timer.tick(time.delta());
        if anim.timer.just_finished() {
            let last = f.len() - 1;
            if anim.current_frame >= last {
                if anim.one_shot {
                    anim.completed = true;
                    continue;
                }
                anim.current_frame = 0;
            } else {
                anim.current_frame += 1;
            }
            sprite.image = f[anim.current_frame].clone();
        }
    }
}
