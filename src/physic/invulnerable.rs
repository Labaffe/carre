use bevy::prelude::*;

/// Marqueur posé sur une entité qui ne doit pas prendre de dégâts.
///
/// Utilisé typiquement pendant les phases de transition d'un boss (cf. boss.rs)
/// ou pendant les frames d'invulnérabilité du joueur (cf. `Invincible` côté
/// player — composant séparé pour ne pas mélanger les sémantiques).
///
/// Les systèmes de dégâts (`projectile_enemy_collision`, `bomb_apply_damage`)
/// vérifient la présence de ce composant et skippent l'application des dégâts.
/// L'entité reste **dangereuse au contact** (le joueur peut toujours être
/// touché par un boss invulnérable qui bouge sur lui).
#[derive(Component, Clone)]
pub struct Invulnerable;
