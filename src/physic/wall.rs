//! Obstacle solide statique. Les entités mobiles avec `NoOverlap` ne peuvent
//! pas le traverser : le `no_overlap_system` push leur position hors du Wall.
//!
//! Visuel : carré noir simple (placeholder). Sera remplacé par un sprite
//! plus tard si besoin.
//!
//! Usage :
//! ```ignore
//! commands.spawn(wall_bundle(Vec2::new(0.0, 0.0), Vec2::new(120.0, 120.0)));
//! ```

use crate::geometry::shape::Shape;
use crate::movement::bounding_radius::BoundingRadius;
use crate::physic::collider::{collider, layers, OverlapEvent};
use crate::physic::no_overlap::{NoOverlap, NoOverlapStatic};
use crate::weapon::projectile::Projectile;
use bevy::prelude::*;

/// Component d'un mur — stocke la demi-taille pour les calculs de push
/// rectangle-vs-cercle (le `BoundingRadius` standard n'autorise que des
/// cercles, ce qui produit des coins "creux" sur les murs carrés).
#[derive(Component)]
#[require(crate::GameplayEntity)]
pub struct Wall {
    pub half_size: Vec2,
}

/// Construit le bundle d'un mur centré sur `position`, de taille `size`.
///
/// - **Wall { half_size }** : utilisé par `wall_push_mobiles_system` pour
///   repousser les entités `NoOverlap` selon la vraie hitbox rectangulaire
///   (pas un cercle inscrit).
/// - **Collider WALL** : détecte les projectiles + ennemis simple_ufo pour
///   despawn au contact via les systèmes correspondants.
///
/// Pas de `NoOverlap` standard ici : le mur est traité spécialement par
/// `wall_push_mobiles_system` (cercle-vs-rect précis).
pub fn wall_bundle(position: Vec2, size: Vec2) -> impl Bundle {
    let half_size = size * 0.5;
    (
        Sprite {
            color: Color::srgb(0.05, 0.05, 0.05),
            custom_size: Some(size),
            ..default()
        },
        Transform::from_xyz(position.x, position.y, 0.45),
        Wall { half_size },
        collider(
            Shape::Rect {
                half_length: size.y * 0.5,
                half_width: size.x * 0.5,
            },
            layers::WALL,
            // Détecte uniquement les projectiles : les ennemis qui n'ont
            // pas `NoOverlap` (simple_ufo Bézier notamment) passent à
            // travers les murs sans être affectés.
            layers::PLAYER_PROJECTILE | layers::ENEMY_PROJECTILE,
        ),
    )
}

/// Pousse chaque entité `NoOverlap` mobile hors de chaque mur, en utilisant
/// la vraie hitbox **rectangle vs cercle** :
/// 1. Calcule le point le plus proche du rect au centre du cercle (clamp)
/// 2. Si distance(centre, point) < rayon → push perpendiculaire au bord
/// 3. Cas dégénéré (cercle entièrement dans le rect) : push selon l'axe le
///    plus court vers la sortie
///
/// Tourne en `PostUpdate` (cf. `NoOverlapPlugin`) pour s'appliquer après
/// tous les mouvements de la frame.
pub fn wall_push_mobiles_system(
    walls_q: Query<(&Transform, &Wall)>,
    mut mobiles_q: Query<
        (&mut Transform, &BoundingRadius),
        (With<NoOverlap>, Without<NoOverlapStatic>, Without<Wall>),
    >,
) {
    for (wall_tf, wall) in walls_q.iter() {
        let wall_pos = wall_tf.translation.truncate();
        let hw = wall.half_size.x;
        let hh = wall.half_size.y;

        for (mut mob_tf, radius) in mobiles_q.iter_mut() {
            let mob_pos = mob_tf.translation.truncate();
            let r = radius.0;

            // Closest point on AABB.
            let dx = (mob_pos.x - wall_pos.x).clamp(-hw, hw);
            let dy = (mob_pos.y - wall_pos.y).clamp(-hh, hh);
            let closest = wall_pos + Vec2::new(dx, dy);
            let diff = mob_pos - closest;
            let dist = diff.length();

            if dist > 0.001 && dist < r {
                // Cas normal : push perpendiculaire au bord du rect.
                let push = diff / dist * (r - dist);
                mob_tf.translation.x += push.x;
                mob_tf.translation.y += push.y;
            } else if dist <= 0.001 {
                // Centre du cercle à l'intérieur du rect : pousser via l'axe
                // de sortie le plus court (manhattan).
                let x_overlap = hw - (mob_pos.x - wall_pos.x).abs();
                let y_overlap = hh - (mob_pos.y - wall_pos.y).abs();
                if x_overlap < y_overlap {
                    let sign = if mob_pos.x >= wall_pos.x { 1.0 } else { -1.0 };
                    mob_tf.translation.x += sign * (x_overlap + r);
                } else {
                    let sign = if mob_pos.y >= wall_pos.y { 1.0 } else { -1.0 };
                    mob_tf.translation.y += sign * (y_overlap + r);
                }
            }
        }
    }
}

/// Détruit tout projectile qui entre en overlap avec un Wall (joueur OU
/// ennemi). Lu une seule fois par `OverlapEvent` — Bevy déduplique les paires.
pub fn wall_destroy_projectiles_system(
    mut commands: Commands,
    mut events: MessageReader<OverlapEvent>,
    projectile_q: Query<(), With<Projectile>>,
) {
    for ev in events.read() {
        let Some((_wall_e, other_e)) = ev.pick(layers::WALL) else {
            continue;
        };
        if projectile_q.get(other_e).is_ok() {
            if let Ok(mut e) = commands.get_entity(other_e) {
                e.try_despawn();
            }
        }
    }
}

