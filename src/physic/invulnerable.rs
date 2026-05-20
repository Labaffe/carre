use bevy::prelude::*;

/// Marqueur posé sur une entité qui ne doit pas prendre de dégâts.
///
/// Utilisé typiquement pendant les phases de transition d'un boss (cf. boss.rs),
/// sur une mine (toujours invulnérable + harmless), ou via F1 (debug mode) sur
/// le joueur. Pour les frames d'invulnérabilité post-hit du joueur, voir
/// le composant séparé `Invincible(Timer)`.
///
/// Le système central `apply_damage` (cf. `physic::health`) vérifie ce
/// composant et skip l'application des dégâts. L'entité reste **dangereuse
/// au contact** (le joueur peut toujours être touché par un boss invulnérable
/// qui bouge sur lui — utiliser `Harmless` pour bloquer le contact).
#[derive(Component, Clone)]
pub struct Invulnerable;
