//! Réactions de collision joueur — système reactive basé sur `OverlapEvent`.
//!
//! Le `Hittable` trait et les `player_collision<T>` génériques ont été retirés
//! au profit du framework unifié [`crate::physic::collider`]. Ce fichier ne
//! garde que :
//! - La constante [`PLAYER_RADIUS`] (utilisée à 1 endroit hors collisions)
//! - Le plugin qui setup les assets AOE + le système réactif de dégâts joueur

use bevy::prelude::*;
use std::time::Duration;

use crate::audio::{Sfx, SfxPlayer};
use crate::debug::debug::DebugMode;
use crate::game_manager::state::GameState;
use crate::physic::area_of_effect::{aoe_lifecycle, setup_aoe_assets};
use crate::physic::collider::{layers, OverlapEvent};
use crate::physic::harmless::Harmless;
use crate::physic::health::Health;
use crate::player::player::{INVINCIBLE_DURATION, Invincible, Player};

pub struct CollisionPlugin;

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_aoe_assets).add_systems(
            Update,
            (player_damage_on_overlap, aoe_lifecycle).run_if(in_state(GameState::Playing)),
        );
    }
}

/// Rayon de la hitbox du joueur (sprite 128x128, hitbox ~70% du demi-côté).
/// Conservé en const pour les usages hors collision (helpers, debug).
pub const PLAYER_RADIUS: f32 = 45.0;

/// Layers d'entités qui INFLIGENT des dégâts au joueur au contact. Sans ce
/// filtre, le système réagirait à n'importe quel overlap impliquant PLAYER
/// (items, etc.) et infligerait des dégâts à tort.
const HOSTILE_TO_PLAYER: u32 =
    layers::ENEMY | layers::ASTEROID | layers::ENEMY_PROJECTILE | layers::AOE;

/// Layers d'entités qui doivent être despawnées quand elles touchent le joueur
/// (consommables au contact). Asteroid se brise, projectile ennemi est consommé.
/// ENEMY et AOE persistent par contre.
const DESPAWN_ON_PLAYER_HIT: u32 = layers::ASTEROID | layers::ENEMY_PROJECTILE;

/// Lit les `OverlapEvent` et applique les dégâts au joueur quand il touche
/// une entité hostile. Réplique exactement le comportement de l'ancien
/// `player_collision<T>` :
/// - Skip si debug mode ou joueur Invincible
/// - Skip les entités `Harmless` (mines, etc.)
/// - Despawn l'hostile si sa layer est dans `DESPAWN_ON_PLAYER_HIT`
/// - Inflige 1 dégât, joue `Sfx::PlayerHurt`
/// - Transition vers GameOver si HP=0, sinon insère Invincible
/// - Early return après 1 hit (cohérent avec l'invincibilité qui kick in)
fn player_damage_on_overlap(
    mut commands: Commands,
    mut events: MessageReader<OverlapEvent>,
    mut player_q: Query<(Entity, &mut Health, Option<&Invincible>), With<Player>>,
    harmless_q: Query<(), With<Harmless>>,
    debug: Res<DebugMode>,
    mut next_state: ResMut<NextState<GameState>>,
    mut sfx: SfxPlayer,
) {
    if debug.0 {
        events.read().for_each(drop);
        return;
    }
    let Ok((player_e, mut health, invincible)) = player_q.single_mut() else {
        events.read().for_each(drop);
        return;
    };
    if invincible.is_some() {
        events.read().for_each(drop);
        return;
    }

    for ev in events.read() {
        let Some((_, hostile)) = ev.pick(layers::PLAYER) else { continue };
        let hostile_layer = if ev.a == hostile { ev.a_layer } else { ev.b_layer };

        // Skip overlaps avec entités non-hostiles (items, etc.).
        if hostile_layer & HOSTILE_TO_PLAYER == 0 { continue; }
        if harmless_q.contains(hostile) { continue; }

        if hostile_layer & DESPAWN_ON_PLAYER_HIT != 0 {
            if let Ok(mut e) = commands.get_entity(hostile) {
                e.try_despawn();
            }
        }

        health.take_damage(1);
        sfx.play_at(Sfx::PlayerHurt, 3.0);

        if health.is_dead() {
            if let Ok(mut e) = commands.get_entity(player_e) {
                e.try_despawn();
            }
            next_state.set(GameState::GameOver);
        } else {
            commands.entity(player_e).insert(Invincible(Timer::new(
                Duration::from_secs_f32(INVINCIBLE_DURATION),
                TimerMode::Once,
            )));
        }
        return;
    }
}
