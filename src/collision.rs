//! Collision joueur ↔ entités hostiles (astéroïdes, boss, projectiles boss).
//! Tout objet implémentant le trait `Hittable` peut tuer le joueur au contact.

use crate::asteroid::Asteroid;
use crate::boss::{Boss, BossProjectile};
use crate::debug::DebugMode;
use crate::missile::Missile;
use crate::player::Player;
use crate::state::GameState;
use crate::weapon::HitboxShape;
use bevy::prelude::*;

pub struct CollisionPlugin;

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                player_collision::<Asteroid>,
                player_collision::<Boss>,
                player_collision::<BossProjectile>,
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
    /// Par défaut true (astéroïdes). Le boss ne meurt pas au contact.
    fn despawn_on_hit(&self) -> bool { true }
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

impl Hittable for Boss {
    fn hitbox_shape(&self) -> HitboxShape {
        HitboxShape::Circle(self.radius)
    }
    fn despawn_on_hit(&self) -> bool { false }
}

impl Hittable for BossProjectile {
    fn hitbox_shape(&self) -> HitboxShape {
        HitboxShape::Circle(self.radius)
    }
}

impl Hittable for Missile {
    fn hitbox_shape(&self) -> HitboxShape {
        self.hitbox.clone()
    }
}

fn player_collision<T: Hittable>(
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    player_q: Query<(Entity, &Transform), With<Player>>,
    hostile_q: Query<(Entity, &Transform, &T)>,
    debug: Res<DebugMode>,
) {
    if debug.0 {
        return;
    }

    let Ok((player_entity, player_transform)) = player_q.get_single() else {
        return;
    };

    for (hostile_entity, hostile_transform, hittable) in hostile_q.iter() {
        let distance = player_transform
            .translation
            .distance(hostile_transform.translation);

        let combined_radius = match hittable.hitbox_shape() {
            HitboxShape::Circle(r) => PLAYER_RADIUS + r,
            HitboxShape::Rect { half_length, half_width } => {
                PLAYER_RADIUS + half_length.max(half_width)
            }
        };

        if distance < combined_radius {
            commands.entity(player_entity).despawn_recursive();
            if hittable.despawn_on_hit() {
                commands.entity(hostile_entity).despawn();
            }
            next_state.set(GameState::GameOver);
            return;
        }
    }
}

