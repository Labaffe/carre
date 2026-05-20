//! GreenUFO — définition data-driven avec spawn system.
//!
//! Cycle de combat : `idle` (2s, immobile) → `rush` (fonce vers le joueur,
//! direction figée au démarrage) → retour en `idle`. Le rush se termine soit
//! par expiration de son timer, soit par contact avec un bord de l'écran
//! (`MovementZone` push `wall_*`). Le green_ufo reste donc TOUJOURS dans
//! l'écran — pas de `DespawnOffScreen`.
//! Mort = instantanée à PV=0 (phase `dying` → `dead` = DespawnSelf).

use std::time::Duration;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::behavior::behavior::BehaviorComponent;
use crate::behavior::choice_list::TransitionMessages;
use crate::behavior::*;
use crate::enemy::anim_bank::Animation;
use crate::enemy::death::DespawnSelf;
use crate::enemy::enemies::GREEN_UFO;
use crate::enemy::enemy::Enemy;
use crate::enemy::enemy_builder::EnemyBuilder;
use crate::game_manager::difficulty::{Difficulty, SpawnPosition};
use crate::item::item::{DropTable, ItemType};
use crate::movement::bounding_radius::BoundingRadius;
use crate::movement::movement_zone::MovementZone;
use crate::movement::movements::Movements;
use crate::movement::rush::Rush;
use crate::geometry::shape::Shape;
use crate::physic::collider::{collider, layers};
use crate::physic::health::Health;
use crate::sprite_orient::FaceMovement;

/// Vitesse du rush (px/s). Pas trop rapide — combiné à `RUSH_DURATION` et
/// `MovementZone` qui interrompt sur contact mur, le green_ufo ne traverse
/// jamais tout l'écran.
const RUSH_SPEED: f32 = 650.0;
/// Durée max d'un rush (secondes). À RUSH_SPEED=600, couvre 480px — distance
/// moyenne, sauf interruption par wall avant.
const RUSH_DURATION: f32 = 1.0;
/// Durée de l'idle entre deux rushes (secondes).
const IDLE_DURATION: f32 = 1.5;
const GREEN_UFO_ANIM_FPS: f32 = 12.0;

static GREEN_UFO_DROP_TABLE: [(ItemType, f32); 2] =
    [(ItemType::Bomb, 0.10), (ItemType::BonusScore, 0.15)];

// ─── Composants ─────────────────────────────────────────────────────

pub struct GreenUFOBuilder {
    timer: Timer,
}
impl GreenUFOBuilder {
    pub fn new() -> Self {
        Self {
            timer: Timer::new(Duration::ZERO, TimerMode::Once),
        }
    }
}
impl EnemyBuilder for GreenUFOBuilder {
    fn get_timer(&mut self) -> &mut Timer {
        &mut self.timer
    }
    fn name(&self) -> &'static str {
        "green_ufo"
    }
    fn preload_anim(&self) -> HashMap<&str, &str> {
        HashMap::from([
            ("green_ufo", "images/green_ufo"),
            ("green_ufo_death", "images/green_ufo/death"),
        ])
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

        // Sub-behavior idle : ne fait rien pendant IDLE_DURATION, puis push
        // "rush_ready" via on_complete → choice transitionne en rush.
        let idle = BehaviorBuilder::first(
            Duration::from_secs_f32(IDLE_DURATION),
            BehaviorBuilder::nothing(),
        )
        .on_complete("rush_ready");

        // Sub-behavior rush : insère Movements(Rush) pendant RUSH_DURATION max.
        // Le Rush fige sa direction (vers le joueur) à la 1re frame. À la fin
        // du timer, on_complete pousse "idle_ready" → retour idle. Si le
        // green_ufo touche un bord avant la fin, MovementZone push "wall_*"
        // → choice interrompt le rush et retourne en idle.
        let rush = BehaviorBuilder::first(
            Duration::from_secs_f32(RUSH_DURATION),
            BehaviorBuilder::from_component(Movements::new().with(Rush::new(RUSH_SPEED))),
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

        // Mort : 10 frames de death animation (one-shot) à 12fps = 0.83s
        // puis DespawnSelf. Movements::new() pour stopper le rush en cours.
        let dying = BehaviorBuilder::first(
            Duration::from_secs_f32(0.83),
            BehaviorBuilder::multiple()
                .with(BehaviorBuilder::from_component(Movements::new()))
                .with(BehaviorBuilder::from_component(
                    Animation::new("green_ufo_death", Duration::from_secs_f32(1.0 / 12.0))
                        .one_shot(),
                )),
        )
        .then(
            Duration::from_secs_f32(0.1),
            BehaviorBuilder::from_component(DespawnSelf),
        );

        let behavior = BehaviorBuilder::choice()
            .with(alive)
            .with(dying)
            .add_transition(0, 1, "die");

        commands.spawn((
            Sprite {
                image: asset_server.load("images/green_ufo/frame000.png"),
                custom_size: Some(Vec2::splat(GREEN_UFO.config.sprite_size)),
                ..default()
            },
            Transform::from_xyz(pos.x, pos.y, 0.5),
            TransitionMessages::new(),
            Enemy::new(GREEN_UFO),
            Health::new(GREEN_UFO.total_hp),
            Animation::new(
                "green_ufo",
                Duration::from_secs_f32(1.0 / GREEN_UFO_ANIM_FPS),
            ),
            BoundingRadius(GREEN_UFO.config.sprite_size / 2.0),
            MovementZone::new(Vec2::ZERO)
                .with_left("wall_left")
                .with_right("wall_right")
                .with_top("wall_top")
                .with_bottom("wall_bottom"),
            BehaviorComponent::new(behavior),
            DropTable {
                drops: &GREEN_UFO_DROP_TABLE,
            },
            FaceMovement::faces_left(),
            collider(
                Shape::Circle(GREEN_UFO.config.radius),
                layers::ENEMY,
                layers::PLAYER | layers::PLAYER_PROJECTILE,
            ),
        ));
    }
}
