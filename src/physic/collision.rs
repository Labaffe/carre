//! Collision joueur ↔ entités hostiles (astéroïdes, ennemis, projectiles ennemis).
//! Tout objet implémentant le trait `Hittable` peut blesser le joueur au contact.

use crate::debug::debug::DebugMode;
use crate::enemy::asteroid::Asteroid;
use crate::enemy::enemy::{Enemy, EnemyState};
use crate::game_manager::state::GameState;
use crate::physic::health::Health;
use crate::player::player::{INVINCIBLE_DURATION, Invincible, Player};
use crate::weapon::projectile::{Projectile, Team};
use crate::weapon::weapon::HitboxShape;
use bevy::prelude::*;
use std::time::Duration;

pub struct CollisionPlugin;

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                player_collision::<Asteroid>,
                player_collision::<Enemy>,
                player_collision::<Projectile>,
            )
                .run_if(in_state(GameState::Playing)),
        );
    }
}

/// Rayon de la hitbox du joueur (sprite 128x128, hitbox ~70% du demi-côté).
pub const PLAYER_RADIUS: f32 = 45.0;

/// Trait commun pour tout objet possédant une hitbox.
pub trait Hittable: Component {
    fn hitbox_shape(&self) -> HitboxShape;
    /// Si true, l'entité hostile est despawnée au contact avec le joueur.
    fn despawn_on_hit(&self) -> bool {
        true
    }
    /// Si false, la collision est ignorée (ex: ennemi en animation d'entrée).
    fn is_dangerous(&self) -> bool {
        true
    }
}

impl Hittable for Player {
    fn hitbox_shape(&self) -> HitboxShape {
        HitboxShape::Circle(PLAYER_RADIUS)
    }
}

impl Hittable for Asteroid {
    fn hitbox_shape(&self) -> HitboxShape {
        HitboxShape::Circle(self.radius)
    }
}

impl Hittable for Enemy {
    fn hitbox_shape(&self) -> HitboxShape {
        HitboxShape::Circle(self.radius)
    }
    fn despawn_on_hit(&self) -> bool {
        false
    }
    fn is_dangerous(&self) -> bool {
        matches!(self.state, EnemyState::Active(_))
    }
}

/// Un `Projectile` ne blesse le joueur que si son `team` est `Enemy`.
/// Les projectiles du joueur sont ignorés par ce système de collision joueur.
impl Hittable for Projectile {
    fn hitbox_shape(&self) -> HitboxShape {
        self.hitbox.clone()
    }
    fn is_dangerous(&self) -> bool {
        self.team == Team::Enemy
    }
}

fn player_collision<T: Hittable>(
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    mut player_q: Query<(Entity, &Transform, &mut Health, Option<&Invincible>), With<Player>>,
    hostile_q: Query<(Entity, &Transform, &T)>,
    debug: Res<DebugMode>,
    asset_server: Res<AssetServer>,
) {
    if debug.0 {
        return;
    }

    let Ok((player_entity, player_transform, mut health, invincible)) = player_q.get_single_mut()
    else {
        return;
    };

    if invincible.is_some() {
        return;
    }

    for (hostile_entity, hostile_transform, hittable) in hostile_q.iter() {
        if !hittable.is_dangerous() {
            continue;
        }

        let distance = player_transform
            .translation
            .distance(hostile_transform.translation);

        let combined_radius = match hittable.hitbox_shape() {
            HitboxShape::Circle(r) => PLAYER_RADIUS + r,
            HitboxShape::Rect {
                half_length,
                half_width,
            } => PLAYER_RADIUS + half_length.max(half_width),
        };

        if distance < combined_radius {
            if hittable.despawn_on_hit() {
                if let Some(mut e) = commands.get_entity(hostile_entity) {
                    e.despawn();
                }
            }

            health.take_damage(1);

            commands.spawn(AudioBundle {
                source: asset_server.load("audio/sfx/hurt.ogg"),
                settings: PlaybackSettings {
                    volume: bevy::audio::Volume::new(3.0),
                    ..PlaybackSettings::DESPAWN
                },
            });

            if health.is_dead() {
                if let Some(e) = commands.get_entity(player_entity) {
                    e.despawn_recursive();
                }
                next_state.set(GameState::GameOver);
            } else {
                commands.entity(player_entity).insert(Invincible(Timer::new(
                    Duration::from_secs_f32(INVINCIBLE_DURATION),
                    TimerMode::Once,
                )));
            }
            return;
        }
    }
}
