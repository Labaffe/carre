//! `EnemyGroup` — entité parent qui coordonne plusieurs enfants `Enemy`.
//!
//! Pattern : on spawne un parent (avec `Movements`, `Transform`, etc.) et
//! on lui attache des enfants via `with_children`. Bevy propage la
//! `GlobalTransform` du parent aux enfants → un mouvement appliqué au
//! parent fait bouger toute la grappe.
//!
//! Le marker `DespawnWhenChildrenEmpty` ajoute la sémantique "le groupe
//! meurt quand toutes ses parties sont mortes" : à chaque frame, on
//! vérifie s'il reste au moins un enfant `Enemy` ; sinon on despawn le
//! parent (la cascade Bevy nettoie les enfants résiduels — sprites
//! restants d'animations de mort en cours, etc.).
//!
//! ## Exemple d'utilisation
//!
//! ```ignore
//! commands.spawn((
//!     EnemyGroup,
//!     DespawnWhenChildrenEmpty,
//!     Movements::new().with(Oscilate::new(...)),
//!     Transform::from_xyz(0.0, 200.0, 0.5),
//!     // Visibility::default() requis si pas de Sprite sur le parent —
//!     // sinon les enfants ne sont pas rendus.
//!     Visibility::default(),
//! ))
//! .with_children(|p| {
//!     p.spawn(turret_bundle(asset_server, Vec2::new(-80.0, 0.0)));
//!     p.spawn(turret_bundle(asset_server, Vec2::new(  0.0, 0.0)));
//!     p.spawn(turret_bundle(asset_server, Vec2::new( 80.0, 0.0)));
//! });
//! ```

use bevy::prelude::*;

use crate::enemy::enemy::Enemy;

/// Marker sur l'entité parent d'un groupe d'ennemis. Purement informatif
/// pour l'instant — peut servir à filtrer dans des queries / debug.
#[derive(Component)]
pub struct EnemyGroup;

/// À combiner avec `EnemyGroup` : le parent despawn quand aucun de ses
/// enfants n'a plus le composant `Enemy` (= tous morts/despawn).
#[derive(Component)]
pub struct DespawnWhenChildrenEmpty;

/// Pour chaque parent `DespawnWhenChildrenEmpty`, vérifie s'il a encore au
/// moins un enfant `Enemy` vivant. Sinon, despawn le parent (cascade
/// supprime les enfants résiduels — par ex. une explosion AOE attachée).
pub fn despawn_empty_groups(
    mut commands: Commands,
    groups: Query<(Entity, &Children), With<DespawnWhenChildrenEmpty>>,
    enemies: Query<(), With<Enemy>>,
) {
    for (group_e, children) in &groups {
        let any_alive = children.iter().any(|c| enemies.contains(c));
        if !any_alive {
            if let Ok(mut e) = commands.get_entity(group_e) {
                e.try_despawn();
            }
        }
    }
}
