pub mod asteroid;
pub mod anim_bank;
pub mod boss;
pub mod enemies;
pub mod enemy;
pub mod enemy_builder;
pub mod enemy_register;
pub mod green_ufo;
pub mod hit_flash;
pub mod kamikaze;
pub mod mine;
pub mod octopus;
pub mod death;
use bevy::prelude::*;
use crate::enemy::anim_bank::*;
use crate::enemy::asteroid::{asteroid_death_fx_system, AsteroidBuilder};
use crate::enemy::boss::{boss_hp_threshold_check, BossBuilder};
use crate::enemy::death::despawn;
use crate::enemy::death::detect_death;
use crate::enemy::enemy::EnemyDeathEvent;
use crate::enemy::enemy::{
    enemy_hit_sound_on_hit, hit_flash_on_hit, projectile_damage_on_overlap, score_on_enemy_hit,
};
use crate::enemy::enemy_register::EnemyRegister;
use crate::enemy::enemy_register::spawn;
use crate::enemy::hit_flash::*;
use crate::enemy::green_ufo::*;
use crate::enemy::kamikaze::{
    kamikaze_boom_system, kamikaze_force_boom_system, kamikaze_laugh_start_system,
    kamikaze_laugh_stop_system, kamikaze_speed_ramp_system, KamikazeBuilder,
};
use crate::enemy::mine::{blink_red_system, mine_countdown_audio, mine_explode_system, MineBuilder};
use crate::enemy::octopus::{
    octopus_become_alive, octopus_die_sound, octopus_fire_shots, octopus_pre_swoop_tick,
    octopus_setup_curve, octopus_telegraph_tick, OctopusBuilder,
};
use crate::GameState;
use crate::menu::pause::not_paused;
pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<EnemyDeathEvent>()
            .insert_resource(
                EnemyRegister::new()
                .with(GreenUFOBuilder::new())
                .with(BossBuilder::new())
                .with(AsteroidBuilder::new())
                .with(MineBuilder::new())
                .with(KamikazeBuilder::new())
                .with(OctopusBuilder::new())
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
                    // Systèmes réactifs (ordre après la machine à état).
                    // `hit_flash_on_hit`, `enemy_hit_sound_on_hit`,
                    // `score_on_enemy_hit` sont maintenant des **observers**
                    // (cf. `add_observer` plus bas) déclenchés par
                    // `commands.trigger(HitEvent)` dans `apply_damage`.
                    projectile_damage_on_overlap,
                    boss_hp_threshold_check,
                    mine_explode_system,
                    mine_countdown_audio,
                    blink_red_system,
                    kamikaze_force_boom_system,
                    kamikaze_boom_system,
                    // kamikaze_scream_system retiré : hook `on_insert` sur `KamikazeArmed`.
                    kamikaze_laugh_start_system,
                    kamikaze_laugh_stop_system,
                    kamikaze_speed_ramp_system,
                    asteroid_death_fx_system,
                )
                    .chain()
                    .run_if(in_state(GameState::Playing))
                    .run_if(not_paused),
            )
            // Observers globaux sur `HitEvent` (trigger par `apply_damage`).
            .add_observer(hit_flash_on_hit)
            .add_observer(enemy_hit_sound_on_hit)
            .add_observer(score_on_enemy_hit)
            // Systèmes Octopus dans leur propre tuple : la limite de `.chain()`
            // (15 systèmes) est atteinte sur le bloc enemy générique au-dessus.
            // Ces systèmes sont tous des réactifs sur `Added<…>` indépendants
            // les uns des autres — pas besoin de chain entre eux.
            .add_systems(
                Update,
                (
                    // Les sons d'apparition (entering_rush, entering_idle),
                    // d'amorçage (shooting) et de tir (fire_shots) sont gérés
                    // par des hooks `on_insert` sur les markers correspondants
                    // dans `octopus.rs` — pas besoin de système dédié.
                    octopus_become_alive,
                    octopus_setup_curve,
                    octopus_fire_shots,
                    octopus_die_sound,
                    octopus_telegraph_tick,
                    octopus_pre_swoop_tick,
                )
                    .run_if(in_state(GameState::Playing))
                    .run_if(not_paused),
            )
            .add_systems(
                Update,
                detect_death.run_if(in_state(GameState::Playing)),
            )
            // despawn dans PostUpdate : garantit que toutes les commandes
            // queuées par update_behavior (cascade de remove::<C> quand un
            // ennemi entre en dying) flushent AVANT le try_despawn. Sinon
            // race condition : try_despawn s'applique en premier, les
            // remove::<C> suivants tapent une entité invalide → panic.
            .add_systems(
                PostUpdate,
                despawn.run_if(in_state(GameState::Playing)),
            )
            ;
    }
}