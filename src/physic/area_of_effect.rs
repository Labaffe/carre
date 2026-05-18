//! Zone de dégâts générique (Area Of Effect).
//!
//! Une entité avec un [`AreaOfEffect`] est un `Hittable` qui touche le joueur
//! au contact mais ne despawn pas (`despawn_on_hit = false`) — elle persiste
//! pour sa `lifetime`. Le joueur prend 1 dégât puis devient `Invincible` pour
//! `INVINCIBLE_DURATION` (=2s), donc en pratique une AOE de lifetime ≤ 2s
//! n'inflige qu'un seul hit.
//!
//! Spawn via [`spawn_aoe`] pour un visuel par défaut (rouge translucide), ou
//! manuellement avec un `Sprite` custom + le composant `AreaOfEffect`.

use bevy::prelude::*;

use crate::geometry::shape::Shape;
use crate::physic::collision::Hittable;

#[derive(Component)]
pub struct AreaOfEffect {
    pub shape: Shape,
    pub lifetime: f32,
    pub elapsed: f32,
}

impl Hittable for AreaOfEffect {
    fn hitbox_shape(&self) -> Shape {
        self.shape.clone()
    }
    /// L'AOE persiste pour toute sa lifetime — le joueur peut entrer/sortir
    /// sans la consommer.
    fn despawn_on_hit(&self) -> bool {
        false
    }
}

/// Tick la lifetime de chaque AOE et despawn quand expirée.
pub fn aoe_lifecycle(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut AreaOfEffect)>,
) {
    let dt = time.delta_secs();
    for (entity, mut aoe) in &mut query {
        aoe.elapsed += dt;
        if aoe.elapsed >= aoe.lifetime {
            commands.entity(entity).try_despawn();
        }
    }
}

/// Helper de spawn : crée une AOE avec un visuel par défaut (sprite rouge
/// translucide dimensionné selon la `shape`). Pour un visuel custom, spawne
/// manuellement avec un `Sprite` choisi + le composant `AreaOfEffect`.
pub fn spawn_aoe(
    commands: &mut Commands,
    position: Vec3,
    shape: Shape,
    lifetime: f32,
) -> Entity {
    let custom_size = match &shape {
        Shape::Circle(r) => Some(Vec2::splat(r * 2.0)),
        Shape::Rect {
            half_length,
            half_width,
        } => Some(Vec2::new(half_width * 2.0, half_length * 2.0)),
    };
    commands
        .spawn((
            Sprite {
                color: Color::srgba(1.0, 0.0, 0.0, 0.3),
                custom_size,
                ..default()
            },
            Transform::from_translation(position),
            AreaOfEffect {
                shape,
                lifetime,
                elapsed: 0.0,
            },
        ))
        .id()
}
