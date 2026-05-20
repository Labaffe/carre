//! Réactions de collision joueur — émetteur de `DamageEvent` + reactive
//! systems sur `HitEvent` pour les conséquences spécifiques au joueur.
//!
//! Le pipeline de dégâts vit dans [`crate::physic::health`]. Ce fichier ne
//! fait QUE détecter les overlaps hostiles et émettre des `DamageEvent`.
//! L'application effective des dégâts (avec filtrage `Invincible`/
//! `Invulnerable`) est centralisée dans `apply_damage`.

use bevy::prelude::*;
use std::time::Duration;

use crate::audio::{Sfx, SfxPlayer};
use crate::debug::debug::DebugMode;
use crate::game_manager::state::GameState;
use crate::physic::area_of_effect::{aoe_damage_enemies_on_overlap, aoe_lifecycle, setup_aoe_assets};
use crate::physic::collider::{layers, OverlapEvent};
use crate::physic::harmless::Harmless;
use crate::physic::health::{DamageEvent, Health, HitEvent};
use crate::player::player::{INVINCIBLE_DURATION, Invincible, Player};

pub struct CollisionPlugin;

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_aoe_assets).add_systems(
            Update,
            (
                player_damage_on_overlap,
                player_post_hit,
                player_hurt_sound_on_hit,
                aoe_damage_enemies_on_overlap,
                aoe_lifecycle,
            )
                .run_if(in_state(GameState::Playing)),
        );
    }
}

/// Rayon de la hitbox du joueur (sprite 128x128, hitbox ~70% du demi-côté).
pub const PLAYER_RADIUS: f32 = 45.0;

/// Layers d'entités qui INFLIGENT des dégâts au joueur au contact.
const HOSTILE_TO_PLAYER: u32 =
    layers::ENEMY | layers::ASTEROID | layers::ENEMY_PROJECTILE | layers::AOE;

/// Layers d'entités qui doivent être despawnées quand elles touchent le joueur
/// (consommables au contact). Asteroid se brise, projectile ennemi est consommé.
/// ENEMY et AOE persistent par contre.
const DESPAWN_ON_PLAYER_HIT: u32 = layers::ASTEROID | layers::ENEMY_PROJECTILE;

/// Émet `DamageEvent` quand le joueur touche une entité hostile. Despawn
/// l'hostile au passage si c'est un type consommable (asteroid, projectile).
fn player_damage_on_overlap(
    mut commands: Commands,
    mut events: MessageReader<OverlapEvent>,
    player_q: Query<Entity, With<Player>>,
    invincible_q: Query<(), With<Invincible>>,
    harmless_q: Query<(), With<Harmless>>,
    debug: Res<DebugMode>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    if debug.0 {
        events.read().for_each(drop);
        return;
    }
    let Ok(player_e) = player_q.single() else {
        events.read().for_each(drop);
        return;
    };
    // Si déjà Invincible, apply_damage skipperait de toute façon. On peut
    // court-circuiter ici pour économiser l'émission d'events inutiles.
    if !invincible_q.is_empty() {
        events.read().for_each(drop);
        return;
    }

    for ev in events.read() {
        let Some((_, hostile)) = ev.pick(layers::PLAYER) else { continue };
        let hostile_layer = if ev.a == hostile { ev.a_layer } else { ev.b_layer };

        if hostile_layer & HOSTILE_TO_PLAYER == 0 { continue; }
        if harmless_q.contains(hostile) { continue; }

        if hostile_layer & DESPAWN_ON_PLAYER_HIT != 0 {
            if let Ok(mut e) = commands.get_entity(hostile) {
                e.try_despawn();
            }
        }
        damage_events.write(DamageEvent {
            target: player_e,
            amount: 1,
            source: Some(hostile),
        });
        return; // 1 hit par frame max (Invincible suit)
    }
}

/// Réagit aux `HitEvent` qui ciblent le joueur : insère `Invincible` ou
/// transitionne vers GameOver selon que le joueur survit ou pas.
fn player_post_hit(
    mut commands: Commands,
    mut events: MessageReader<HitEvent>,
    health_q: Query<&Health, With<Player>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for ev in events.read() {
        if ev.target_layer & layers::PLAYER == 0 { continue; }
        let Ok(health) = health_q.get(ev.target) else { continue };

        if health.is_dead() {
            if let Ok(mut e) = commands.get_entity(ev.target) {
                e.try_despawn();
            }
            next_state.set(GameState::GameOver);
        } else {
            if let Ok(mut e) = commands.get_entity(ev.target) {
                e.try_insert(Invincible(Timer::new(
                    Duration::from_secs_f32(INVINCIBLE_DURATION),
                    TimerMode::Once,
                )));
            }
        }
    }
}

/// Joue le son de dégât joueur sur HitEvent ciblant PLAYER.
fn player_hurt_sound_on_hit(
    mut events: MessageReader<HitEvent>,
    mut sfx: SfxPlayer,
) {
    for ev in events.read() {
        if ev.target_layer & layers::PLAYER != 0 {
            sfx.play_at(Sfx::PlayerHurt, 3.0);
        }
    }
}
