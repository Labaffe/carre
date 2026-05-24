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
use crate::movement::bezier::Bezier;
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
/// Durée totale de l'animation de mort (s) — durée par frame recalculée
/// auto via `with_total_duration`.
const GREEN_UFO_DEATH_DURATION: f32 = 0.45;

/// Entrée intangible le long d'une Bézier (depuis off-screen vers
/// l'intérieur de l'écran). Sprite teinté, pas de collider.
const ENTERING_DURATION: f32 = 1.0;
/// Distance verticale parcourue pendant l'entering.
const ENTERING_DESCENT: f32 = 350.0;
/// Teinte d'intangibilité (sombre + alpha réduit) pendant l'entering.
const INTANGIBLE_TINT: Color = Color::srgba(0.35, 0.35, 0.35, 0.9);

static GREEN_UFO_DROP_TABLE: [(ItemType, f32); 3] = [
    (ItemType::Bomb, 0.10),
    (ItemType::BonusScore, 0.15),
    (ItemType::Armor, 0.08),
];

/// Marker présent pendant l'entering (descente Bézier intangible).
#[derive(Component, Clone)]
pub struct GreenUfoEntering;

/// Marker inséré à la fin de l'entering. `green_ufo_become_alive` détecte
/// `Added<_>` pour insérer le collider et restaurer l'alpha.
#[derive(Component, Clone)]
pub struct GreenUfoAlive;

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
        let entry_pos = spawn_pos.resolve(window, 60.0);
        // Cible : descente d'`ENTERING_DESCENT` depuis le spawn off-screen,
        // X aléatoire dans le tiers central → varie chaque spawn.
        let half_w = window.width() / 2.0;
        let target_x = (fastrand::f32() - 0.5) * 2.0 * (half_w * 0.4);
        let final_pos = Vec2::new(target_x, entry_pos.y - ENTERING_DESCENT);
        // Point de passage à mi-chemin, fortement décalé latéralement pour
        // créer une courbe visible (passing_through impose t=0.5 par ce point).
        let swerve_x = (fastrand::f32() - 0.5) * window.width() * 0.6;
        let mid_pos = Vec2::new(swerve_x, (entry_pos.y + final_pos.y) * 0.5);

        // Entering : Bézier (entry → mid → final), sans collider, sprite
        // teinté. Aucun message à pousser pendant — le `first` timer expiré
        // déclenche "entering_done".
        let entering = BehaviorBuilder::first(
            Duration::from_secs_f32(ENTERING_DURATION),
            BehaviorBuilder::multiple()
                .with(BehaviorBuilder::from_component(GreenUfoEntering))
                .with(BehaviorBuilder::from_component(
                    Movements::new().with(Bezier::passing_through(
                        entry_pos,
                        mid_pos,
                        final_pos,
                        Duration::from_secs_f32(ENTERING_DURATION),
                    )),
                )),
        )
        .on_complete("entering_done");

        let idle = BehaviorBuilder::first(
            Duration::from_secs_f32(IDLE_DURATION),
            BehaviorBuilder::nothing(),
        )
        .on_complete("rush_ready");

        let rush = BehaviorBuilder::first(
            Duration::from_secs_f32(RUSH_DURATION),
            BehaviorBuilder::from_component(Movements::new().with(Rush::new(RUSH_SPEED))),
        )
        .on_complete("idle_ready");

        let alive_cycle = BehaviorBuilder::choice()
            .with(idle) // 0
            .with(rush) // 1
            .add_transition(0, 1, "rush_ready")
            .add_transition(1, 0, "idle_ready")
            .add_transition(1, 0, "wall_left")
            .add_transition(1, 0, "wall_right")
            .add_transition(1, 0, "wall_top")
            .add_transition(1, 0, "wall_bottom");

        let alive = BehaviorBuilder::multiple()
            .with(BehaviorBuilder::from_component(GreenUfoAlive))
            .with(alive_cycle);

        let dying = BehaviorBuilder::first(
            Duration::from_secs_f32(GREEN_UFO_DEATH_DURATION),
            BehaviorBuilder::multiple()
                .with(BehaviorBuilder::from_component(Movements::new()))
                .with(BehaviorBuilder::from_component(
                    Animation::with_total_duration(
                        "green_ufo_death",
                        Duration::from_secs_f32(GREEN_UFO_DEATH_DURATION),
                    )
                    .one_shot(),
                )),
        )
        .then(
            Duration::from_secs_f32(0.1),
            BehaviorBuilder::from_component(DespawnSelf),
        );

        let behavior = BehaviorBuilder::choice()
            .with(entering) // 0
            .with(alive) // 1
            .with(dying) // 2
            .add_transition(0, 1, "entering_done")
            .add_transition(1, 2, "die");

        commands.spawn((
            Sprite {
                image: asset_server.load("images/green_ufo/frame000.png"),
                custom_size: Some(Vec2::splat(GREEN_UFO.config.sprite_size)),
                // Teinte sombre pendant entering — restaurée par
                // `green_ufo_become_alive` à l'entrée du state alive.
                color: INTANGIBLE_TINT,
                ..default()
            },
            Transform::from_xyz(entry_pos.x, entry_pos.y, 0.5),
            TransitionMessages::new(),
            Enemy::new(GREEN_UFO),
            Health::new(GREEN_UFO.total_hp),
            Animation::new(
                "green_ufo",
                Duration::from_secs_f32(1.0 / GREEN_UFO_ANIM_FPS),
            ),
            BoundingRadius(GREEN_UFO.config.sprite_size / 2.0),
            crate::physic::no_overlap::NoOverlap,
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
            // PAS de collider ici : intangible pendant entering.
            // `green_ufo_become_alive` (Added<GreenUfoAlive>) l'insère à
            // la fin de l'entering.
        ));
    }
}

/// `Added<GreenUfoAlive>` : fin de l'entering Bézier. Restaure l'alpha
/// du sprite et insère le collider — l'ennemi devient tangible.
pub fn green_ufo_become_alive(
    mut commands: Commands,
    mut q: Query<(Entity, &mut Sprite), Added<GreenUfoAlive>>,
) {
    for (entity, mut sprite) in &mut q {
        sprite.color = Color::WHITE;
        if let Ok(mut e) = commands.get_entity(entity) {
            e.try_insert(collider(
                Shape::Circle(GREEN_UFO.config.radius),
                layers::ENEMY,
                layers::PLAYER | layers::PLAYER_PROJECTILE,
            ));
        }
    }
}
