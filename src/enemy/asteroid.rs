//! Système d'astéroïdes : spawn aléatoire, mouvement, flash au hit.
//!
//! Les textures sont scannées dynamiquement depuis `assets/images/asteroids/`.
//! La taille, la vitesse et les PV dépendent du diamètre généré aléatoirement.
//! La vélocité de base est stockée sans le facteur de difficulté :
//! celui-ci est appliqué chaque frame dans `move_asteroids`, ce qui permet
//! aux astéroïdes déjà à l'écran d'accélérer quand la difficulté augmente.
use bevy::platform::collections::HashMap;

use crate::behavior::choice_list::TransitionMessages;
use crate::behavior::*;
use crate::behavior::behavior::{Behavior, BehaviorComponent};
use crate::enemy::anim_bank::Animation;
use crate::enemy::death::DespawnSelf;
use crate::movement::despawn_off_screen::DespawnOffScreen;
use crate::enemy::enemy::Enemy;
use crate::enemy::enemy_builder::EnemyBuilder;
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


#[derive(Component)]
pub struct Asteroid {
    pub radius: f32,
    pub size: Vec2,
}

/// Info conservée pour spawner l'animation d'explosion à la mort de
/// l'astéroïde. Stocke l'ID du sprite (pour trouver le dossier
/// `images/asteroids/death_x{NNN}/`) et la vitesse de chute (pour que
/// l'explosion conserve l'élan de l'astéroïde).
#[derive(Component)]
pub struct AsteroidDeathFx {
    pub texture_index: usize,
    pub size: Vec2,
    pub velocity: Vec3,
}



pub struct AsteroidBuilder {
    timer:Timer
}
impl AsteroidBuilder {
    pub fn new()-> Self {Self {timer:Timer::new(Duration::ZERO,TimerMode::Once)}}
}

impl EnemyBuilder for AsteroidBuilder {
    fn preload_anim(&self)->bevy::platform::collections::HashMap<&str, &str> {
        HashMap::from([
            ("asteroid2", "images/asteroids/asteroid2"),
            ("asteroid3", "images/asteroids/asteroid3"),
            ("asteroid4", "images/asteroids/asteroid4"),
            ("asteroid5", "images/asteroids/asteroid5"),
            ("asteroid6", "images/asteroids/asteroid6"),
            ("asteroid7", "images/asteroids/asteroid7"),
            ("asteroid8", "images/asteroids/asteroid8"),
            ("asteroid9", "images/asteroids/asteroid9"),
            ("asteroid10", "images/asteroids/asteroid10"),
            ("asteroid12", "images/asteroids/asteroid12"),
            ("asteroid13", "images/asteroids/asteroid13"),
            ("asteroid14", "images/asteroids/asteroid14"),
            ("asteroid15", "images/asteroids/asteroid15"),
            ("asteroid16", "images/asteroids/asteroid16")
        ])
    }

    fn get_timer(&mut self)->&mut Timer {
        &mut self.timer
    }

    fn spawn(
        &self,
        mut commands: Commands,
        window: &Window,
        difficulty: &ResMut<Difficulty>,
        spawn_pos: crate::game_manager::difficulty::SpawnPosition,
        asset_server:&Res<AssetServer>) {
        let pos = spawn_pos.resolve(window, 60.0);
        let half_w = window.width() / 2.0;
        let half_h = window.height() / 2.0;
        let x = fastrand::f32() * window.width() - half_w;
        let is_small = fastrand::f32() < 0.3; // 30% de petits, 70% de gros
        // IDs des sprites présents dans `assets/images/asteroids/` — note les
        // trous (pas d'asteroid11) qui sortaient un sprite vide avant.
        const ASTEROID_IDS: &[usize] = &[2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 13, 14, 15, 16];
        let pick = ASTEROID_IDS[fastrand::usize(0..ASTEROID_IDS.len())];
        let texture_name = "asteroid".to_string() + &pick.to_string();

        let anim = Animation::new(&texture_name,Duration::from_secs_f32(5.0));
        // Spawn au-dessus de l'écran (juste hors du champ visible)
        let transform = Transform::from_xyz(pos.x, pos.y + 100.0, 0.0).with_rotation(
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
        
        let alive = BehaviorBuilder::from_component(Movements::new()
            .with(Translate::new(Vec2::new(0.0,-1.0), speed*difficulty.factor)));
        let dying = BehaviorBuilder::from_component(DespawnSelf);
        let behavior = BehaviorBuilder::choice()
            .with(alive)
            .with(dying)
            .add_transition(0,1,"die");
        commands.spawn((
            Sprite { custom_size: Some(size), ..default() },
            transform,
            anim,
            Enemy {radius,sprite_size:size.x,name:"Asteroid"},
            Health::new(health),
            DropTable {
                drops: &ASTEROID_DROP_TABLE,
            },
            TransitionMessages::new(),
            DespawnOffScreen,
            AsteroidDeathFx {
                texture_index: pick,
                size,
                velocity: base_velocity,
            },
            BehaviorComponent::new(behavior),
        ));
    }

    fn name(&self)->&str {
        "asteroid"
    }
}

/// Sur réception d'un `EnemyDeathEvent`, si l'entité morte est un astéroïde
/// (a `AsteroidDeathFx`), spawn une explosion à sa position. Utilise
/// `spawn_explosion` qui cherche `images/asteroids/death_x{NNN}/`, fallback
/// sur l'explosion générique si le dossier custom n'existe pas.
pub fn asteroid_death_fx_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut events: bevy::ecs::message::MessageReader<crate::enemy::enemy::EnemyDeathEvent>,
    asteroid_q: Query<(&Transform, &AsteroidDeathFx)>,
) {
    for event in events.read() {
        let Ok((tf, fx)) = asteroid_q.get(event.entity) else { continue };
        crate::fx::explosion::spawn_explosion(
            &mut commands,
            &asset_server,
            tf.translation,
            fx.size,
            fx.texture_index,
            fx.velocity,
            tf.rotation,
        );
    }
}
