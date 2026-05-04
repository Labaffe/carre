//! Système d'astéroïdes : spawn aléatoire, mouvement, flash au hit.
//!
//! Les textures sont scannées dynamiquement depuis `assets/images/asteroids/`.
//! La taille, la vitesse et les PV dépendent du diamètre généré aléatoirement.
//! La vélocité de base est stockée sans le facteur de difficulté :
//! celui-ci est appliqué chaque frame dans `move_asteroids`, ce qui permet
//! aux astéroïdes déjà à l'écran d'accélérer quand la difficulté augmente.

use crate::behavior::*;
use crate::behavior::behavior::Behavior;
use crate::enemy::despawn_zone::DespawnZone;
use crate::movement::movements::Movements;
use crate::behavior::ordered_list::OrderedNodeList;
use crate::game_manager::difficulty::Difficulty;
use crate::game_manager::state::GameState;
use crate::item::item::{DropTable, ItemType};
use crate::movement::translate::Translate;
use crate::physic::health::Health;
use bevy::prelude::*;
use std::time::Duration;
use crate::enemy::hit_flash;
/// Table de drop des astéroïdes : 5% bombe, 10% bonus score.
static ASTEROID_DROP_TABLE: [(ItemType, f32); 2] = [
    (ItemType::Bomb, 0.05),
    (ItemType::BonusScore, 0.10),
];

pub struct AsteroidPlugin;

impl Plugin for AsteroidPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AsteroidSpawner::default())
            .add_systems(Startup, preload_asteroid_textures)
            .add_systems(
                Update,
                (spawn_asteroids)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

#[derive(Component)]
pub struct Asteroid {
    pub base_velocity: Vec3,
    pub radius: f32,
    pub size: Vec2,
    /// Index de la texture (0-16), utilisé pour trouver le dossier d'animation de mort.
    pub texture_index: usize,
}

/// Inséré sur l'astéroïde au moment d'un hit. Le timer contrôle la durée du flash.
#[derive(Component)]
pub struct HitFlash(pub Timer);

#[derive(Resource)]
struct AsteroidTextures(Vec<(usize, Handle<Image>)>);

#[derive(Resource)]
struct AsteroidSpawner {
    timer: Timer,
}

impl Default for AsteroidSpawner {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(1.0, TimerMode::Repeating),
        }
    }
}

fn preload_asteroid_textures(mut commands: Commands, asset_server: Res<AssetServer>) {
    let dir = std::path::Path::new("assets/images/asteroids");
    let mut entries = Vec::new();

    if let Ok(read_dir) = std::fs::read_dir(dir) {
        for entry in read_dir.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            // Cherche les fichiers asteroid_xNNN.png
            if let Some(rest) = name.strip_prefix("asteroid_x") {
                if let Some(num_str) = rest.strip_suffix(".png") {
                    if let Ok(index) = num_str.parse::<usize>() {
                        let handle = asset_server.load(format!("images/asteroids/{}", name));
                        entries.push((index, handle));
                    }
                }
            }
        }
    }

    entries.sort_by_key(|(i, _)| *i);
    commands.insert_resource(AsteroidTextures(entries));
}

fn spawn_asteroids(
    windows: Query<&Window>,
    mut commands: Commands,
    mut spawner: ResMut<AsteroidSpawner>,
    time: Res<Time>,
    difficulty: Res<Difficulty>,
    textures: Res<AsteroidTextures>,
) {
    // Spawn uniquement si "asteroid" est dans les spawners actifs
    if !difficulty.active_spawners.contains_key("asteroid") {
        return;
    }

    spawner
        .timer
        .set_duration(Duration::from_secs_f32(difficulty.spawn_interval()));
    spawner.timer.tick(time.delta());

    if !spawner.timer.just_finished() {
        return;
    }

    let window = windows.single();
    let half_w = window.width() / 2.0;
    let half_h = window.height() / 2.0;
    let x = fastrand::f32() * window.width() - half_w;
    let is_small = fastrand::f32() < 0.3; // 30% de petits, 70% de gros
    let pick = fastrand::usize(..textures.0.len());
    let (texture_index, ref texture) = textures.0[pick];
    let texture = texture.clone();

    // Spawn au-dessus de l'écran (juste hors du champ visible)
    let transform = Transform::from_xyz(x, half_h + 100.0, 0.0).with_rotation(
        Quat::from_rotation_z(fastrand::f32() * std::f32::consts::TAU),
    );

    // Taille : petits 80-110px, gros 120-180px
    let side = if is_small {
        fastrand::f32() * 30.0 + 80.0
    } else {
        fastrand::f32() * 60.0 + 120.0
    };
    let size = Vec2::splat(side);
    let radius = side * 0.30; // hitbox = 30% du diamètre

    // PV proportionnels à la taille : 1 (petit) à 5 (très gros)
    let health = if side < 35.0 {
        1
    } else {
        ((side - 35.0) / (180.0 - 35.0) * 4.0 + 1.0)
            .round()
            .clamp(1.0, 5.0) as i32
    };

    // Vitesse inversement proportionnelle à la taille (petits = rapides)
    // Stockée sans le facteur de difficulté (appliqué dans move_asteroids)
    // Petits ~250 px/s, gros ~100 px/s
    let speed = 250.0 - (side - 35.0) / (180.0 - 35.0) * 150.0;
    let base_velocity = Vec3::new(0.0, -speed, 0.0);
    
    let movements_slow = Movements::new()
        .with(Translate::new(Vec2::new(0.0,-1.0), speed));
    let movements_fast = Movements::new()
        .with(Translate::new(Vec2::new(0.0,-1.0), speed*3.5));
    let behavior=BehaviorBuilder::choice()
        .with(BehaviorBuilder::from_component(movements_slow))
        .with(BehaviorBuilder::from_component(movements_fast));

    commands.spawn((
        SpriteBundle {
            texture,
            sprite: Sprite {
                custom_size: Some(size),
                ..default()
            },
            transform,
            ..default()
        },
        Asteroid {
            base_velocity,
            radius,
            size,
            texture_index,
        },
        Health::new(health),
        DropTable {
            drops: &ASTEROID_DROP_TABLE,
        },
        DespawnZone {
            x:-window.width(),
            y:-window.height(),
            width: window.width() * 2,
            height:window.height() * 0.25
        },
        behavior
    ));
}