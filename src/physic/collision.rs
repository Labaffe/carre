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
use crate::enemy::death::DespawnSelf;
use crate::game_manager::state::GameState;
use crate::physic::area_of_effect::{aoe_damage_enemies_on_overlap, aoe_lifecycle, setup_aoe_assets};
use crate::physic::collider::{layers, OverlapEvent};
use crate::physic::harmless::Harmless;
use crate::physic::health::{DamageEvent, Health, HitEvent};
use crate::player::player::{INVINCIBLE_DURATION, Invincible, Player};
use crate::ui::score::{Combo, Score};

pub struct CollisionPlugin;

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_aoe_assets)
            .add_systems(
                Update,
                (
                    player_damage_on_overlap,
                    aoe_damage_enemies_on_overlap,
                    aoe_lifecycle,
                )
                    .run_if(in_state(GameState::Playing)),
            )
            // Observers globaux sur `HitEvent` (trigger par `apply_damage`).
            .add_observer(play_player_hurt_sfx)
            .add_observer(player_post_hit);
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
///
/// Note : pas de check `DebugMode` ici. F1 insère le composant `Invulnerable`
/// sur le joueur, ce qui fait que `apply_damage` skip le dégât naturellement.
fn player_damage_on_overlap(
    mut commands: Commands,
    mut events: MessageReader<OverlapEvent>,
    player_q: Query<Entity, With<Player>>,
    invincible_q: Query<(), With<Invincible>>,
    harmless_q: Query<(), With<Harmless>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
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
            // DespawnSelf au lieu de try_despawn direct : évite la race avec
            // les commandes du behavior tree (cas d'un asteroid qui meurt
            // simultanément par missile → cascade disable). Le système
            // `despawn` en PostUpdate dépile après le flush de Update.
            if let Ok(mut e) = commands.get_entity(hostile) {
                e.try_insert(DespawnSelf);
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

/// Observer : réagit aux `HitEvent` ciblant le joueur. Insère `Invincible`
/// ou transitionne vers GameOver selon que le joueur survit ou pas.
/// Reset aussi le combo (un hit absorbé par l'armor reset également, car
/// `apply_damage` trigger `HitEvent` dès qu'au moins 1 point est encaissé).
fn player_post_hit(
    trigger: On<HitEvent>,
    mut commands: Commands,
    health_q: Query<&Health, With<Player>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut score: ResMut<Score>,
    mut combo: ResMut<Combo>,
) {
    let ev = trigger.event();
    if ev.target_layer & layers::PLAYER == 0 { return; }
    let Ok(health) = health_q.get(ev.target) else { return };

    combo.reset(&mut score);

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

/// Observer global : joue le son de dégât joueur sur `HitEvent` ciblant
/// la layer `PLAYER`. Fire en synchrone quand `apply_damage` trigger l'event,
/// pas besoin d'un système qui poll chaque frame.
fn play_player_hurt_sfx(trigger: On<HitEvent>, mut sfx: SfxPlayer) {
    let ev = trigger.event();
    if ev.target_layer & layers::PLAYER != 0 {
        sfx.play_at(Sfx::PlayerHurt, 3.0);
    }
}
