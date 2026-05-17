pub mod asteroid;
pub mod anim_bank;
pub mod boss;
pub mod enemies;
pub mod enemy;
pub mod enemy_builder;
pub mod enemy_register;
pub mod green_ufo;
pub mod hit_flash;
pub mod despawn_zone;
//pub mod spawn;
mod death;
use bevy::prelude::*;
use crate::enemy::anim_bank::*;
use crate::enemy::asteroid::AsteroidBuilder;
use crate::enemy::boss::BossBuilder;
use crate::enemy::death::despawn;
use crate::enemy::death::detect_death;
use crate::enemy::enemy::EnemyDeathEvent;
use crate::enemy::enemy::projectile_enemy_collision;
use crate::enemy::enemy_register::EnemyRegister;
use crate::enemy::enemy_register::spawn;
use crate::enemy::hit_flash::*;
use crate::enemy::green_ufo::*;
use crate::GameState;
use crate::menu::pause::not_paused;
pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<EnemyDeathEvent>()
            .insert_resource(
                EnemyRegister::new()
                .with(GreenUFOBuilder::new())
                .with(BossBuilder::new())
                .with(AsteroidBuilder::new())
            )
            .insert_resource(AnimBank::new())
            .add_systems(Startup, preload_frames)
            .add_systems(
                Update,
                (
                    animate_hit_flash,
                    animate,
                    spawn
                )
                .run_if(in_state(GameState::Playing))
                .run_if(not_paused))
            .add_systems(
                Update,
                (
                    // Framework phases+behaviors (exclusif, séquentiel)
                    // Systèmes réactifs (ordre après la machine à état)
                    projectile_enemy_collision,
                )
                    .chain()
                    .run_if(in_state(GameState::Playing))
                    .run_if(not_paused),
            ).add_systems(
                Update,
                ( detect_death,despawn)
                    .run_if(in_state(GameState::Playing)),
            )
            ;
    }
}