use bevy::prelude::*;

/// Marqueur posé sur une entité qui ne doit pas prendre de dégâts.
///
/// Utilisé typiquement pendant les phases de transition d'un boss (cf. boss.rs),
/// sur une mine (toujours invulnérable + harmless), ou par les pouvoirs du
/// joueur (shield, dash). Pour les frames d'invulnérabilité post-hit du
/// joueur, voir le composant séparé `Invincible(Timer)`.
///
/// Le système central `apply_damage` (cf. `physic::health`) vérifie ce
/// composant et skip l'application des dégâts. L'entité reste **dangereuse
/// au contact** (le joueur peut toujours être touché par un boss invulnérable
/// qui bouge sur lui — utiliser `Harmless` pour bloquer le contact).
#[derive(Component, Clone)]
pub struct Invulnerable;

/// Marqueur équivalent à `Invulnerable` mais géré **exclusivement** par le
/// mode debug (F1). Séparé pour que le toggle F1 ne touche pas aux
/// `Invulnerable` posés par d'autres systèmes (shield, dash, mine, boss).
///
/// Sans cette séparation, `debug_player_invulnerability` retire le
/// composant à chaque frame quand F1=off, ce qui annule l'invulnérabilité
/// d'un shield/dash active. `apply_damage` skip aussi sur ce marker.
#[derive(Component, Clone)]
pub struct DebugInvulnerable;
