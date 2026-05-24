//! Simple UFO Shooter — ennemi mobile-tireur.
//!
//! Cycle : `idle` (1.5s, immobile) → `rush` (translate dans une **direction
//! aléatoire** pendant `RUSH_DURATION`, ET amorce une rafale de 3 projectiles
//! successifs **vers le joueur** au début du rush) → retour `idle`. Rush
//! interrompu sur contact bord via `MovementZone` qui push `wall_*`.
//!
//! Différences-clés avec [`green_ufo`] :
//! - Direction du rush = random (pas vers le joueur)
//! - Rafale de 3 tirs en LIGNE DROITE (direction figée au début du rush)
//! - Sprite orienté en permanence vers le joueur (via `FacePlayer` local)
//!
//! Le sprite de base regarde vers le HAUT (+Y local), donc l'angle de
//! rotation final = `atan2(dir.y, dir.x) - π/2` (cf. convention `spawn_projectile`).

use std::time::Duration;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::behavior::behavior::BehaviorComponent;
use crate::behavior::choice_list::TransitionMessages;
use crate::behavior::*;
use crate::enemy::death::DespawnSelf;
use crate::enemy::enemies::SIMPLE_UFO_SHOOTER;
use crate::enemy::enemy::Enemy;
use crate::enemy::enemy_builder::EnemyBuilder;
use crate::game_manager::difficulty::{Difficulty, SpawnPosition};
use crate::geometry::shape::Shape;
use crate::item::item::{DropTable, ItemType};
use crate::movement::bounding_radius::BoundingRadius;
use crate::movement::movement::Movement;
use crate::movement::movement_zone::MovementZone;
use crate::movement::movements::Movements;
use crate::physic::collider::{collider, layers};
use crate::physic::health::Health;
use crate::player::player::Player;
use crate::weapon::projectile::{spawn_projectile, ProjectileSpawn, ProjectileSprite, Team};

// ─── Constantes ──────────────────────────────────────────────────────

const RUSH_SPEED: f32 = 550.0;
const RUSH_DURATION: f32 = 0.8;
const IDLE_DURATION: f32 = 1.5;

const PROJECTILE_COUNT: i32 = 3;
/// Intervalle entre 2 projectiles de la rafale (s). Total burst ≈ 0.3s.
const BURST_INTERVAL: f32 = 0.1;
const PROJECTILE_SPEED: f32 = 600.0;
const PROJECTILE_SIZE: Vec2 = Vec2::new(8.0, 18.0);
const PROJECTILE_COLOR: Color = Color::srgb(0.4, 0.9, 1.0);

static SIMPLE_UFO_SHOOTER_DROP_TABLE: [(ItemType, f32); 3] = [
    (ItemType::Bomb, 0.10),
    (ItemType::BonusScore, 0.15),
    (ItemType::Armor, 0.08),
];

// ─── Movement custom : RandomRush ────────────────────────────────────

/// Translate dans une direction aléatoire fixée à la 1re frame d'évaluation.
/// Identique en structure à `Rush` mais la direction n'est pas dérivée du
/// joueur.
#[derive(Clone)]
pub struct RandomRush {
    speed: f32,
    already_set: bool,
    direction: Vec2,
}

impl RandomRush {
    pub fn new(speed: f32) -> Self {
        Self { speed, already_set: false, direction: Vec2::ZERO }
    }
}

impl Movement for RandomRush {
    fn evaluate(
        &mut self,
        _at: Duration,
        deltatime: Duration,
        _current_position: Vec2,
        _velocity: Vec2,
        _player_pos: Vec2,
    ) -> Vec2 {
        if !self.already_set {
            let angle = fastrand::f32() * std::f32::consts::TAU;
            self.direction = Vec2::new(angle.cos(), angle.sin());
            self.already_set = true;
        }
        self.direction * self.speed * deltatime.as_secs_f32()
    }
    fn clone_box(&self) -> Box<dyn Movement + Send + Sync> {
        Box::new(self.clone())
    }
}

// ─── Composants ──────────────────────────────────────────────────────

/// Aligne en continu le sprite vers le joueur (sprite de base regarde +Y).
#[derive(Component)]
pub struct FacePlayer;

/// Inséré par le BT au début du rush, retiré à sa sortie.
/// Le système `simple_ufo_shooter_fire_system` détecte `Added<_>` et amorce
/// un `ShooterBurst` indépendant (qui survit même si le rush se termine
/// avant la fin de la rafale).
#[derive(Component, Clone)]
pub struct SimpleUfoShooterFire;

/// Rafale en cours : tire 1 projo par tick du `timer` dans `direction`
/// (fixée au moment du déclenchement → 3 projectiles en ligne droite).
/// Auto-retiré quand `remaining = 0`.
#[derive(Component)]
pub struct ShooterBurst {
    remaining: i32,
    timer: Timer,
    direction: Vec2,
}

// ─── Builder ─────────────────────────────────────────────────────────

pub struct SimpleUfoShooterBuilder {
    timer: Timer,
}

impl SimpleUfoShooterBuilder {
    pub fn new() -> Self {
        Self { timer: Timer::new(Duration::ZERO, TimerMode::Once) }
    }
}

impl EnemyBuilder for SimpleUfoShooterBuilder {
    fn get_timer(&mut self) -> &mut Timer {
        &mut self.timer
    }
    fn name(&self) -> &'static str {
        "simple_ufo_shooter"
    }
    fn preload_anim(&self) -> HashMap<&str, &str> {
        HashMap::new()
    }
    fn spawn(
        &self,
        mut commands: Commands,
        window: &Window,
        _difficulty: &ResMut<Difficulty>,
        spawn_pos: SpawnPosition,
        asset_server: &Res<AssetServer>,
    ) {
        let pos = spawn_pos.resolve(window, 60.0);

        // Idle : insère le marker `SimpleUfoShooterFire` → la rafale de 3
        // tirs vers le joueur part au DÉBUT de l'idle (ennemi immobile,
        // facilement lisible). Le marker est retiré à la sortie d'idle.
        let idle = BehaviorBuilder::first(
            Duration::from_secs_f32(IDLE_DURATION),
            BehaviorBuilder::from_component(SimpleUfoShooterFire),
        )
        .on_complete("rush_ready");

        // Rush : déplacement aléatoire pur, sans tir.
        let rush = BehaviorBuilder::first(
            Duration::from_secs_f32(RUSH_DURATION),
            BehaviorBuilder::from_component(
                Movements::new().with(RandomRush::new(RUSH_SPEED)),
            ),
        )
        .on_complete("idle_ready");

        let alive = BehaviorBuilder::choice()
            .with(idle) // 0
            .with(rush) // 1
            .add_transition(0, 1, "rush_ready")
            .add_transition(1, 0, "idle_ready")
            .add_transition(1, 0, "wall_left")
            .add_transition(1, 0, "wall_right")
            .add_transition(1, 0, "wall_top")
            .add_transition(1, 0, "wall_bottom");

        let dying = BehaviorBuilder::first(
            Duration::from_secs_f32(0.05),
            BehaviorBuilder::from_component(Movements::new()),
        )
        .then(
            Duration::from_secs_f32(0.05),
            BehaviorBuilder::from_component(DespawnSelf),
        );

        let behavior = BehaviorBuilder::choice()
            .with(alive)
            .with(dying)
            .add_transition(0, 1, "die");

        commands.spawn((
            Sprite {
                image: asset_server.load("images/simple_ufo_shooter.png"),
                custom_size: Some(Vec2::splat(SIMPLE_UFO_SHOOTER.config.sprite_size)),
                ..default()
            },
            Transform::from_xyz(pos.x, pos.y, 0.5),
            TransitionMessages::new(),
            Enemy::new(SIMPLE_UFO_SHOOTER),
            Health::new(SIMPLE_UFO_SHOOTER.total_hp),
            BoundingRadius(SIMPLE_UFO_SHOOTER.config.sprite_size / 2.0),
            MovementZone::new(Vec2::ZERO)
                .with_left("wall_left")
                .with_right("wall_right")
                .with_top("wall_top")
                .with_bottom("wall_bottom"),
            BehaviorComponent::new(behavior),
            DropTable { drops: &SIMPLE_UFO_SHOOTER_DROP_TABLE },
            FacePlayer,
            collider(
                Shape::Circle(SIMPLE_UFO_SHOOTER.config.radius),
                layers::ENEMY,
                layers::PLAYER | layers::PLAYER_PROJECTILE,
            ),
        ));
    }
}

// ─── Systèmes ────────────────────────────────────────────────────────

/// Aligne `Transform.rotation` pour que +Y local pointe vers le joueur.
/// Convention identique à `spawn_projectile` (sprite "vers le haut" → angle
/// `atan2(y, x) - π/2`).
pub fn face_player_system(
    mut q: Query<&mut Transform, (With<FacePlayer>, Without<Player>)>,
    player_q: Query<&Transform, With<Player>>,
) {
    let Ok(player_tf) = player_q.single() else { return };
    let player_pos = player_tf.translation.truncate();
    for mut tf in q.iter_mut() {
        let to_player = player_pos - tf.translation.truncate();
        if to_player.length_squared() < 0.0001 {
            continue;
        }
        let angle = to_player.y.atan2(to_player.x) - std::f32::consts::FRAC_PI_2;
        tf.rotation = Quat::from_rotation_z(angle);
    }
}

/// Au début du rush (`Added<SimpleUfoShooterFire>`), pose un `ShooterBurst`
/// avec la direction figée vers le joueur à ce moment précis. Le timer est
/// initialisé "déjà expiré" pour que le 1er tir parte à la frame suivante.
pub fn simple_ufo_shooter_fire_system(
    mut commands: Commands,
    shooter_q: Query<(Entity, &Transform), Added<SimpleUfoShooterFire>>,
    player_q: Query<&Transform, With<Player>>,
) {
    let Ok(player_tf) = player_q.single() else { return };
    let player_pos = player_tf.translation.truncate();
    for (entity, shooter_tf) in &shooter_q {
        let to_player = player_pos - shooter_tf.translation.truncate();
        let dir = to_player.normalize_or_zero();
        if dir == Vec2::ZERO {
            continue;
        }
        let mut timer = Timer::from_seconds(BURST_INTERVAL, TimerMode::Repeating);
        // Précharge le timer pour que le 1er tir parte dès la prochaine tick.
        let dur = timer.duration();
        timer.set_elapsed(dur);
        if let Ok(mut e) = commands.get_entity(entity) {
            e.try_insert(ShooterBurst {
                remaining: PROJECTILE_COUNT,
                timer,
                direction: dir,
            });
        }
    }
}

/// Tick chaque rafale, spawn 1 projo dans la direction figée à chaque
/// expiration du timer. Retire le composant quand `remaining = 0`.
pub fn shooter_burst_tick(
    time: Res<Time>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut q: Query<(Entity, &Transform, &mut ShooterBurst)>,
) {
    for (entity, tf, mut burst) in q.iter_mut() {
        burst.timer.tick(time.delta());
        if !burst.timer.just_finished() {
            continue;
        }
        let origin = tf.translation;
        let dir = burst.direction;
        spawn_projectile(
            &mut commands,
            &*asset_server,
            ProjectileSpawn {
                position: Vec3::new(origin.x, origin.y, 0.55),
                direction: dir,
                speed: PROJECTILE_SPEED,
                hitbox: Shape::Rect {
                    half_length: PROJECTILE_SIZE.y / 2.0,
                    half_width: PROJECTILE_SIZE.x / 2.0,
                },
                team: Team::Enemy,
                damage: 1,
                sprite: ProjectileSprite::Colored {
                    color: PROJECTILE_COLOR,
                    size: PROJECTILE_SIZE,
                },
                death_folder: None,
            },
        );
        burst.remaining -= 1;
        if burst.remaining <= 0 {
            if let Ok(mut e) = commands.get_entity(entity) {
                e.try_remove::<ShooterBurst>();
            }
        }
    }
}
