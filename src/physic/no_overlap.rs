//! Anti-chevauchement doux entre entités opt-in via le marker `NoOverlap`.
//!
//! À chaque frame, pour chaque paire `(A, B)` d'entités portant `NoOverlap`
//! et `BoundingRadius`, si la distance centre-à-centre est inférieure à la
//! somme des rayons, on applique une séparation :
//! - **Mobile vs mobile** : push 50/50 dans des directions opposées
//! - **Mobile vs Static** : push 100% sur le mobile (Static n'est jamais
//!   déplacé — pratique pour les murs, obstacles fixes)
//! - **Static vs Static** : skip (ne se déplace ni l'un ni l'autre)
//!
//! C'est une séparation **soft** : le mouvement principal (Movements,
//! input joueur, etc.) reprend la main au tick suivant — on évite juste le
//! chevauchement complet.
//!
//! Coût : O(n²) sur le nombre d'entités marquées. Le marker est opt-in
//! pour éviter d'inclure les très nombreuses entités (projectiles, items).

use crate::game_manager::state::GameState;
use crate::menu::pause::not_paused;
use crate::movement::bounding_radius::BoundingRadius;
use bevy::prelude::*;

pub struct NoOverlapPlugin;

impl Plugin for NoOverlapPlugin {
    fn build(&self, app: &mut App) {
        // PostUpdate : tourne après tous les mouvements (Update du joueur +
        // FixedUpdate du movement_driver). Garantit que la correction
        // anti-overlap voit les positions finales et n'est pas re-écrasée
        // par un mouvement subséquent (ce qui causait des ennemis en rush
        // qui traversaient les murs).
        app.add_systems(
            PostUpdate,
            (
                no_overlap_system,
                crate::physic::wall::wall_push_mobiles_system,
            )
                .run_if(in_state(GameState::Playing))
                .run_if(not_paused),
        );
    }
}

/// Marker opt-in : les entités ainsi marquées sont mutuellement séparées
/// dès qu'elles chevauchent (selon leur `BoundingRadius`).
#[derive(Component)]
pub struct NoOverlap;

/// Marker additionnel : "masse infinie", ne bouge jamais lors d'une
/// séparation. À combiner avec `NoOverlap` sur les obstacles fixes (murs,
/// décor solide). Sans ce marker, l'entité est traitée comme mobile.
#[derive(Component)]
pub struct NoOverlapStatic;

fn no_overlap_system(
    mut query: Query<(Entity, &mut Transform, &BoundingRadius, Has<NoOverlapStatic>), With<NoOverlap>>,
) {
    // Snapshot positions + rayons + flag static en 1re passe pour pouvoir
    // muter en 2e passe sans conflit d'emprunt.
    let snapshot: Vec<(Entity, Vec2, f32, bool)> = query
        .iter()
        .map(|(e, tf, r, is_static)| (e, tf.translation.truncate(), r.0, is_static))
        .collect();
    if snapshot.len() < 2 {
        return;
    }

    let mut deltas: Vec<Vec2> = vec![Vec2::ZERO; snapshot.len()];
    for i in 0..snapshot.len() {
        for j in (i + 1)..snapshot.len() {
            let (_, p_a, r_a, static_a) = snapshot[i];
            let (_, p_b, r_b, static_b) = snapshot[j];

            // Skip si les deux sont statiques (rien ne bouge).
            if static_a && static_b {
                continue;
            }

            let diff = p_b - p_a;
            let dist = diff.length();
            let min_dist = r_a + r_b;
            if dist >= min_dist {
                continue;
            }

            // Direction normalisée (cas dégénéré : pousser sur X arbitraire).
            let dir = if dist > 0.001 {
                diff / dist
            } else {
                Vec2::new(1.0, 0.0)
            };
            let overlap = if dist > 0.001 { min_dist - dist } else { min_dist };

            // Répartition selon les statuts static :
            // - mobile vs mobile : 50/50
            // - mobile vs static : 100% sur le mobile
            let (push_a, push_b) = match (static_a, static_b) {
                (false, false) => (overlap * 0.5, overlap * 0.5),
                (true, false) => (0.0, overlap),
                (false, true) => (overlap, 0.0),
                (true, true) => unreachable!(),
            };
            deltas[i] -= dir * push_a;
            deltas[j] += dir * push_b;
        }
    }

    for (i, (_, mut tf, _, _)) in query.iter_mut().enumerate() {
        let d = deltas[i];
        if d != Vec2::ZERO {
            tf.translation.x += d.x;
            tf.translation.y += d.y;
        }
    }
}
