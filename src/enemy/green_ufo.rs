//! GreenUFO — définition data-driven avec spawn system.
//!
//! Phases : `rush` (fonce vers le joueur) ↔ `idle` (pause) en boucle.
//! Mort = instantanée à PV=0 (phase `dying` → `dead` = DespawnSelf).

use std::str::FromStr;
use std::time::Duration;

use bevy::prelude::*;
use bevy::utils::hashbrown::HashMap;

use crate::behavior::*;
use crate::behavior::behavior::BehaviorComponent;
use crate::enemy::anim_bank::Animation;

use crate::enemy::death::DespawnSelf;
use crate::enemy::despawn_zone::DespawnZone;

use crate::enemy::enemies::GREEN_UFO;
use crate::enemy::enemy::Enemy;
use crate::enemy::enemy_builder::EnemyBuilder;
use crate::enemy::hit_flash::HitFlash;
use crate::game_manager::difficulty::{Difficulty, SpawnPosition};
use crate::item::item::{DropTable, ItemType};
use crate::movement::chase::Chase;
use crate::movement::movements::Movements;
use crate::movement::rush::Rush;
use crate::movement::translate::{ Translate};
use crate::physic::health::Health;

const RUSH_SPEED: f32 = 800.0;
const RUSH_DURATION: f32 = 1.5;
const IDLE_DURATION: f32 = 1.0;
const GREEN_UFO_ANIM_FPS: f32 = 12.0;
const GREEN_UFO_SPAWN_INTERVAL: f32 = 2.0;

static GREEN_UFO_DROP_TABLE: [(ItemType, f32); 2] =
    [(ItemType::Bomb, 0.10), (ItemType::BonusScore, 0.15)];

// ─── Composants ─────────────────────────────────────────────────────



pub struct GreenUFOBuilder {
    timer:Timer
}
impl GreenUFOBuilder {
    pub fn new()-> Self {Self {timer:Timer::new(Duration::ZERO,TimerMode::Once)}}
}
impl EnemyBuilder for GreenUFOBuilder {
    fn get_timer(&mut self)->&mut Timer {
        &mut self.timer
    }
    fn name(&self)->&str {
        "green_ufo"
    }
    fn preload_anim(&self)->HashMap<&str, &str> {
        HashMap::from([
            ("green_ufo", "images/green_ufo"),
            ("green_ufo_death", "images/green_ufo/death")
        ])
    }
    fn spawn(
        &self,
        mut commands: Commands,
        window: &Window,
        difficulty: &ResMut<Difficulty>,
        spawn_pos: SpawnPosition,
        asset_server: &Res<AssetServer>
    ) {
        let pos = spawn_pos.resolve(window, 60.0);
        //let first_frame = frames.0.first().cloned().unwrap_or_default();
        let rush_movement = Movements::new()
            .with(Rush::new(10.0))
            ;
        let rush_sound = BehaviorBuilder::nothing();
        // AudioBundle {
        //        source: asset_server.load("audio/sfx/green_ufo.ogg"),
        //        settings: PlaybackSettings::DESPAWN,
        //    };
        let alive=BehaviorBuilder::first(
            Duration::from_secs_f32(RUSH_DURATION),
            BehaviorBuilder::multiple()
                .with(rush_sound)
                .with(BehaviorBuilder::from_component(rush_movement))
        ).then(
            Duration::from_secs_f32(IDLE_DURATION),
            BehaviorBuilder::nothing()
        ).should_loop();
        let dying_sound = BehaviorBuilder::nothing();
        //from_component(AudioBundle {
        //        source: asset_server.load("audio/sfx/green_ufo_death.ogg"),
        //        settings: PlaybackSettings::DESPAWN,
        //});
        let dying_animation = BehaviorBuilder::from_component(Animation::new("green_ufo",Duration::from_secs_f32(1.0 / GREEN_UFO_ANIM_FPS)));

        let dying=BehaviorBuilder::first(
            Duration::from_secs_f32(0.4),
            BehaviorBuilder::multiple()
                .with(dying_sound)
        ).then(
            Duration::from_secs_f32(1.0),
            BehaviorBuilder::from_component(DespawnSelf)
        );
        let behavior = BehaviorBuilder::choice()
            .with(alive)
            .with(dying);
        commands.spawn((
            SpriteBundle {
                //texture: first_frame,
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(GREEN_UFO.config.sprite_size)),
                    ..default()
                },
                transform: Transform::from_xyz(pos.x, pos.y, 0.5),
                ..default()
            },
            Enemy::new(GREEN_UFO),
            Health::new(GREEN_UFO.total_hp),
            HitFlash (Timer::new(Duration::from_secs_f32(1.0), TimerMode::Once) ),
            Animation::new("green_ufo",Duration::from_secs_f32(1.0 / GREEN_UFO_ANIM_FPS)),
            DespawnZone {
                x:-window.width(),
                y:-window.height(),
                width: window.width() * 2.0,
                height:window.height() * 0.25
            }, //todo make it more precise
            BehaviorComponent::new(behavior),
            DropTable {
                drops: &GREEN_UFO_DROP_TABLE,
            }
        ));
    }
}

