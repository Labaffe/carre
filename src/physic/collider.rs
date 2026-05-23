//! Détection de collision unifiée par layers et masks.
//!
//! ## Principe
//!
//! Le système est découpé en 2 couches :
//!
//! 1. **Détection spatiale** (ce module) : un seul système central
//!    [`detect_overlaps`] qui parcourt toutes les paires d'entités collidables,
//!    vérifie l'overlap géométrique, et émet un [`OverlapEvent`] par paire qui
//!    se touche ET dont les masks autorisent la détection.
//!
//! 2. **Réactions métier** (systèmes ailleurs) : chaque règle de gameplay
//!    (joueur prend des dégâts, projectile tue un ennemi, item ramassé, etc.)
//!    devient un système qui *lit* les events et filtre par layer.
//!
//! ## Composants
//!
//! Chaque entité collidable porte 3 composants :
//! - [`Hitbox`] : la forme (cercle / rect orienté) utilisée pour la détection.
//! - [`CollisionLayer`] : la catégorie à laquelle l'entité appartient (UN seul
//!   bit dans le bitflag).
//! - [`CollidesWith`] : un masque indiquant quelles layers cette entité veut
//!   détecter (bitwise OR de plusieurs bits possible).
//!
//! Une collision A↔B est rapportée si `mask_A & layer_B ≠ 0` OU `mask_B & layer_A ≠ 0`.
//!
//! ## Layers disponibles
//!
//! Voir le module [`layers`]. Conventions :
//! - PLAYER : le vaisseau joueur (1 instance)
//! - PLAYER_PROJECTILE : tirs du joueur
//! - ENEMY : ennemis (boss, kamikaze, mine, green_ufo)
//! - ASTEROID : astéroïdes (terrain destructible neutre)
//! - ENEMY_PROJECTILE : tirs ennemis (réservé, futur)
//! - ITEM : items ramassables
//! - AOE : zones de dégâts persistantes
//!
//! Note : les zones de détection (mine/kamikaze armed via `PlayerDetection`)
//! restent gérées par leur système dédié — pas migrées dans ce framework car
//! elles ont une sémantique enter/exit avec cooldown.

use bevy::prelude::*;

use crate::geometry::shape::{shapes_overlap, Shape};

// ─── Bitflags de layers ──────────────────────────────────────────────

pub mod layers {
    pub const PLAYER: u32 = 1 << 0;
    pub const PLAYER_PROJECTILE: u32 = 1 << 1;
    pub const ENEMY: u32 = 1 << 2;
    pub const ENEMY_PROJECTILE: u32 = 1 << 3;
    pub const ASTEROID: u32 = 1 << 4;
    pub const ITEM: u32 = 1 << 5;
    pub const AOE: u32 = 1 << 6;
}

// ─── Composants ──────────────────────────────────────────────────────

/// Forme de collision de l'entité. Position et rotation viennent de la
/// `Transform` de l'entité.
#[derive(Component, Clone)]
pub struct Hitbox(pub Shape);

/// Catégorie de l'entité (un seul bit). Utilise les constantes de [`layers`].
#[derive(Component, Clone, Copy)]
pub struct CollisionLayer(pub u32);

/// Masque des layers que cette entité veut détecter. Bitwise OR pour combiner.
#[derive(Component, Clone, Copy)]
pub struct CollidesWith(pub u32);

// ─── Event ───────────────────────────────────────────────────────────

/// Émis par [`detect_overlaps`] pour chaque paire d'entités qui se touchent et
/// dont au moins une a un mask incluant la layer de l'autre.
///
/// L'ordre `a`/`b` n'est pas garanti — les systèmes réactifs filtrent par
/// layer pour identifier "qui est quoi".
#[derive(Message, Clone, Copy, Debug)]
pub struct OverlapEvent {
    pub a: Entity,
    pub b: Entity,
    pub a_layer: u32,
    pub b_layer: u32,
    /// Point milieu entre les 2 centres — approximation de point de contact.
    pub position: Vec2,
}

impl OverlapEvent {
    /// Helper : si l'event implique un certain layer, retourne (entity_de_ce_layer,
    /// autre_entity). Sinon `None`.
    pub fn pick(&self, target_layer: u32) -> Option<(Entity, Entity)> {
        if self.a_layer & target_layer != 0 {
            Some((self.a, self.b))
        } else if self.b_layer & target_layer != 0 {
            Some((self.b, self.a))
        } else {
            None
        }
    }

    /// Helper : si l'event est exactement entre les 2 layers donnés (dans un
    /// ordre quelconque), retourne (entity_de_layer_a, entity_de_layer_b).
    pub fn pick_pair(&self, layer_a: u32, layer_b: u32) -> Option<(Entity, Entity)> {
        if self.a_layer & layer_a != 0 && self.b_layer & layer_b != 0 {
            Some((self.a, self.b))
        } else if self.b_layer & layer_a != 0 && self.a_layer & layer_b != 0 {
            Some((self.b, self.a))
        } else {
            None
        }
    }
}

// ─── Système central de détection ────────────────────────────────────

/// Parcourt toutes les paires d'entités collidables (N²) et émet un
/// [`OverlapEvent`] pour chaque paire qui se touche ET dont les masks
/// autorisent la détection mutuelle.
///
/// Complexité : O(N²) sur le nombre d'entités collidables présentes. Suffisant
/// jusqu'à ~50 entités simultanées. Au-delà, ajouter un index spatial (grid
/// uniforme ou quadtree) — l'API event reste la même.
pub fn detect_overlaps(
    // `GlobalTransform` (et pas `Transform`) : indispensable pour que les
    // entités parentées (ex: tourelles enfants d'un `EnemyGroup` vaisseau)
    // soient testées à leur position **monde**, pas à leur offset local.
    // Sans ça, le sprite suit le parent mais la hitbox reste collée à
    // l'origine locale → on tire à travers la tourelle.
    q: Query<(Entity, &GlobalTransform, &Hitbox, &CollisionLayer, &CollidesWith)>,
    mut events: MessageWriter<OverlapEvent>,
) {
    let items: Vec<_> = q.iter().collect();
    for i in 0..items.len() {
        let (e_a, gt_a, hb_a, lay_a, mask_a) = items[i];
        let pos_a = gt_a.translation().xy();
        let rot_a = gt_a.rotation();
        for j in (i + 1)..items.len() {
            let (e_b, gt_b, hb_b, lay_b, mask_b) = items[j];

            // Intérêt mutuel : au moins l'un des deux veut détecter l'autre.
            let interested = (mask_a.0 & lay_b.0 != 0) || (mask_b.0 & lay_a.0 != 0);
            if !interested {
                continue;
            }

            let pos_b = gt_b.translation().xy();
            let rot_b = gt_b.rotation();
            if shapes_overlap(pos_a, rot_a, &hb_a.0, pos_b, rot_b, &hb_b.0) {
                events.write(OverlapEvent {
                    a: e_a,
                    b: e_b,
                    a_layer: lay_a.0,
                    b_layer: lay_b.0,
                    position: (pos_a + pos_b) * 0.5,
                });
            }
        }
    }
}

// ─── Plugin ──────────────────────────────────────────────────────────

pub struct ColliderPlugin;

impl Plugin for ColliderPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<OverlapEvent>().add_systems(
            Update,
            detect_overlaps.run_if(in_state(crate::game_manager::state::GameState::Playing)),
        );
    }
}

// ─── Constructeurs d'aide ────────────────────────────────────────────

/// Bundle pratique pour spawner une entité collidable d'un coup. Sucre
/// syntaxique : `Collider::new(shape, layer, mask)` produit le triplet
/// `(Hitbox, CollisionLayer, CollidesWith)`.
pub fn collider(
    shape: Shape,
    layer: u32,
    mask: u32,
) -> (Hitbox, CollisionLayer, CollidesWith) {
    (
        Hitbox(shape),
        CollisionLayer(layer),
        CollidesWith(mask),
    )
}
