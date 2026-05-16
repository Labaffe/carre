//! Boss — définition data-driven basée sur le framework `enemy/system.rs`.
//!
//! ## Phases
//! ```
//! entering ──timer 7s──→ active_1 ──HP<66%──→ transitioning_1 ──timer 2s──→ active_2
//!                                                                              │
//!                                                                          HP<33%
//!                                                                              ▼
//!     dead ←──on_enter despawn── dying ←──HP<1%── active_3 ←──timer 2s── transitioning_2
//! ```
//!
//! ## Parités à valider en jeu (vs ancien boss.rs)
//! - **Intro** : spirale + scaling — l'easing "progress²" matche
//! - **Musique boss** : démarre sur `on_enter active_1` (pas de délai progressif
//!   comme avant avec `boss_music_delayed`). Si tu veux le délai, ajouter une
//!   phase `idle` intermédiaire de 0.5s entre intro et active_1.
//! - **Charge** : `PatrolAndCharge` déclenche une charge tous les N secondes.
//!   L'ancien boss synchronisait au pattern (patrol 5s → charge 0.1s → patrol).
//!   La cadence est proche mais le timing peut différer de ±0.5s.
//! - **Transitions** : shake + flash OK. Spawn d'UFOs idem.
//! - **Mort** : DyingFx fait shake+flash, les **explosions aléatoires**
//!   pendant la mort ne sont PAS spawnées (limitation des behaviors &mut World).
//!   → flaggé en `TODO-VISUEL`.
//! - **Animation idle** (cycle de frames sur le sprite boss) : pas encore
//!   implémentée. Le boss reste sur `frame000.png` en Phase1/2/3.
//!   → flaggé en `TODO-VISUEL`.

use std::time::Duration;

use bevy::prelude::*;

use crate::behavior::BehaviorBuilder;
use crate::behavior::behavior::BehaviorComponent;
use crate::enemy::anim_bank::Animation;
use crate::movement::goto::{self, Goto};
use crate::movement::movement::Movement;
use crate::movement::oscilate::Oscilate;
use crate::movement::rotate::RotateAround;
use crate::movement::rush::{self, Rush};
use crate::movement::shake::Shake;
use crate::movement::sinusoid::Sinusoid;
use crate::movement::translate::Translate;
use bevy::utils::hashbrown::HashMap;
use crate::enemy::enemies::BOSS;
use crate::enemy::enemy::Enemy;
use crate::enemy::death::DespawnSelf;
use crate::enemy::enemy_builder::EnemyBuilder;
use crate::game_manager::difficulty::{Difficulty, SpawnPosition};
use crate::game_manager::state::GameState;
use crate::menu::pause::not_paused;
use crate::movement::movements::Movements;
use crate::physic::health::Health;
use crate::player::player::Player;

// ═══════════════════════════════════════════════════════════════════════
//  Marqueurs (utilisés par boss.rs pour le charge_movement + musique)
// ═══════════════════════════════════════════════════════════════════════

/// Marqueur présent sur l'entité boss (utilisé pour la musique + charge).
#[derive(Component)]
pub struct BossMarker;

/// Composant indiquant que le boss charge le joueur dans une direction figée.
/// Le système `boss_charge_movement` (dans boss.rs) déplace l'entité tant
/// que ce composant est présent.
#[derive(Component)]
pub struct BossCharge {
    pub direction: Vec2,
}

/// Marqueur pour la musique du boss.
#[derive(Component)]
pub struct MusicBoss;

// ═══════════════════════════════════════════════════════════════════════
//  Constantes
// ═══════════════════════════════════════════════════════════════════════

const INTRO_DURATION: f32 = 7.0;
const INTRO_TARGET_Y: f32 = 250.0;
const INTRO_START_SCALE: f32 = 0.01;
const INTRO_END_SCALE: f32 = 1.0;
const INTRO_SPIRAL_TURNS: f32 = 2.5;
const INTRO_SPIRAL_RADIUS: f32 = 150.0;

const PHASE1_PATROL_SPEED_X: f32 = 200.0;
const PHASE2_PATROL_SPEED_X: f32 = 270.0;
const PHASE3_PATROL_SPEED_X: f32 = 270.0;
const PATROL_SINE_AMPLITUDE: f32 = 0.85;
const PATROL_SINE_FREQ: f32 = 4.5;
const PATROL_MARGIN: f32 = 80.0;

const CHARGE_SPEED_P1: f32 = 1500.0;
const CHARGE_SPEED_P2: f32 = 2000.0;
const CHARGE_SPEED_P3: f32 = 2500.0;

const TRANSITION_DURATION: f32 = 2.0;
const TRANSITION_SHAKE: f32 = 12.0;
const TRANSITION_UFO_COUNT_1: usize = 2;
const TRANSITION_UFO_COUNT_2: usize = 4;

const DYING_DURATION: f32 = 4.0;
const DYING_SHAKE_MAX: f32 = 20.0;

// ═══════════════════════════════════════════════════════════════════════
//  Définition du boss
// ═══════════════════════════════════════════════════════════════════════

pub struct BossBuilder{
    timer:Timer
}
impl BossBuilder {
    pub fn new()-> Self {Self {timer:Timer::new(Duration::ZERO,TimerMode::Once)}}
}
impl EnemyBuilder for BossBuilder {
    fn get_timer(&mut self)->&mut Timer {
        &mut self.timer
    }

    fn preload_anim(&self)->HashMap<&str, &str> {
        HashMap::from([
            ("boss", "images/boss/animation_1"),
            ("boss_flexing", "images/boss/flexing"),
            ("boss_idle", "images/boss/idle"),
        ])
    }

    fn spawn(&self,
        mut commands: Commands,
        window: &Window,
        difficulty: &ResMut<Difficulty>,
        spawn_pos: SpawnPosition,
        asset_server:&Res<AssetServer>
    ) {
        println!("going to spawn boss");
        let spiral = Movements::new()
        .with(RotateAround::new(Vec2::ZERO, INTRO_SPIRAL_TURNS))
        .with(Goto::new(Vec2::ZERO,-100.0));
        let entering = BehaviorBuilder::first(
            Duration::from_secs_f32(1.0),
            BehaviorBuilder::multiple()
            .with(BehaviorBuilder::from_component(spiral))
            .with(BehaviorBuilder::from_component(Animation::new("idle", Duration::from_secs(1))))
        ).then(
            Duration::from_secs(1),
            BehaviorBuilder::from_component(Animation::new("boss_flexing",Duration::from_secs_f32(0.1)))
        );
            //.with(BehaviorBuilder::from_component(AudioBundle {
            //    source: asset_server.load("audio/sfx/boss_start.ogg"),
            //    settings: PlaybackSettings::DESPAWN,
        //}));
        let boss_anim_behavior = BehaviorBuilder::from_component( Animation::new("boss", Duration::from_secs_f32(0.1)));
        let boss_idle_anim_behavior = BehaviorBuilder::from_component( Animation::new("boss_idle", Duration::from_secs_f32(0.1)));

        let patrol_movement_left=Movements::new()
            .with(Oscilate::new(
                Vec2::new(1.0, 0.0),
                Vec2::ZERO,
                6.0, 
                window.height() *0.5
            ))
            .with(Translate::new(Vec2::new(-1.0,0.0), 100.0));
        let patrol_movement_right=Movements::new()
            .with(Oscilate::new(
                Vec2::new(1.0, 0.0),
                Vec2::ZERO,
                6.0, 
                window.height() *0.5
            ))
            .with(Translate::new(Vec2::new(1.0,0.0), 100.0));
        let charge_movement1 =BehaviorBuilder::from_component(Movements::new().with( Rush::new( 1500.0)));
        let charge_movement2 =BehaviorBuilder::from_component(Movements::new().with( Rush::new( 1500.0)));
        let alive= BehaviorBuilder::first(
                Duration::from_secs_f32(6.0), 
                BehaviorBuilder::multiple()
                .with(BehaviorBuilder::from_component( patrol_movement_left))
                .with(boss_idle_anim_behavior)   
                )
            .then(
                Duration::from_secs_f32(1.0),
                BehaviorBuilder::multiple()
                .with(charge_movement1)
                .with(boss_anim_behavior)
            )
            .then(
                Duration::from_secs_f32(6.0),
                BehaviorBuilder::multiple()
                .with(
                BehaviorBuilder::from_component( patrol_movement_right))
            )
            .then(
                Duration::from_secs_f32(1.0),
                charge_movement2
            )
            .should_loop();
        let dying=BehaviorBuilder::first(
            Duration::from_secs_f32(0.4),
            BehaviorBuilder::from_component(Movements::new().with(Shake::new(100.0,0.4)))
        ).then(
            Duration::from_secs_f32(1.0),
            BehaviorBuilder::from_component(DespawnSelf)
        );
        let life = BehaviorBuilder::choice()
            .with(alive)
            .with(dying)
            .add_transition(0, 1, "die");
        let behavior = BehaviorBuilder::first(
            Duration::from_secs_f32(2.0), 
            entering
        ).then(
            Duration::from_secs_f32(10000.0),
            life
        );
        commands.spawn((
        SpriteBundle {
            texture: asset_server.load("images/boss/idle/frame000.png"),
            sprite: Sprite {
                custom_size: Some(Vec2::splat(BOSS.config.sprite_size)),
                color: Color::WHITE,
                ..default()
            },
            transform: Transform {
                translation: Vec3::ZERO,
                scale: Vec3::splat(INTRO_END_SCALE),
                ..default()
            },
            ..default()
        },

        Enemy::new(BOSS),
        Health::new(BOSS.total_hp),
        BossMarker,
        BehaviorComponent::new( behavior)
    ));
    }

    fn name(&self)->&str {
        "boss"
    }
}
