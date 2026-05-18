use bevy::prelude::*;

/// Marqueur posé sur une entité hostile qui ne doit pas faire de dégâts au
/// joueur au contact.
///
/// Distinct de [`crate::physic::invulnerable::Invulnerable`] — les deux
/// sémantiques sont indépendantes :
/// - `Invulnerable` : l'entité ne **prend pas** de dégâts.
/// - `Harmless` : l'entité ne **fait pas** de dégâts.
///
/// Le boss en entering (spirale d'arrivée) porte les deux. Pour un cas où
/// les deux divergent : un ennemi qui finit son animation de mort pourrait
/// être `Harmless` (ne touche plus le joueur) mais déjà mort donc non
/// queryable, ou un ennemi en charge prudente qui prend des dégâts mais ne
/// touche pas — peu réaliste mais l'API le permet.
///
/// Filtre appliqué côté `player_collision` via `Without<Harmless>` sur la
/// query des entités hostiles.
#[derive(Component, Clone)]
pub struct Harmless;
