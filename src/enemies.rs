//! Registre central de tous les ennemis du jeu.
//!
//! Chaque ennemi est défini par un `EnemyDef` : ses stats, ses phases,
//! son apparence et ses sons. Pour ajouter un nouvel ennemi :
//! 1. Définir ses constantes et ses `PhaseDef` ici
//! 2. Ajouter son `EnemyDef` dans la liste `ALL_ENEMIES`
//! 3. Créer son module de systèmes spécifiques (intro, patterns, etc.)
//!
//! Les systèmes génériques (dégâts, flash, mort, projectiles, patrol)
//! fonctionnent automatiquement via `EnemyPlugin` dans `enemy.rs`.

use crate::enemy::{PatternDef, PhaseDef};

// ═══════════════════════════════════════════════════════════════════════
//  Définition générique d'un ennemi
// ═══════════════════════════════════════════════════════════════════════

/// Fiche descriptive d'un type d'ennemi.
///
/// Contient toutes les données nécessaires pour spawner et configurer
/// un `Enemy` component. Les systèmes spécifiques (intro, patterns)
/// restent dans le module dédié de chaque ennemi.
pub struct EnemyDef {
    /// Nom affiché (debug / logs).
    pub name: &'static str,
    /// Rayon de la hitbox circulaire.
    pub radius: f32,
    /// Taille du sprite (côté, en pixels).
    pub sprite_size: f32,
    /// Phases de combat (PV, intervalle, patterns, son).
    pub phases: &'static [PhaseDef],
    /// Durée de l'animation de mort (secondes).
    pub death_duration: f32,
    /// Amplitude max du tremblement pendant la mort.
    pub death_shake_max: f32,
    /// Son joué quand l'ennemi est touché.
    pub hit_sound: &'static str,
    /// Son des explosions pendant la mort.
    pub death_explosion_sound: &'static str,
}

// ═══════════════════════════════════════════════════════════════════════
//  BOSS
// ═══════════════════════════════════════════════════════════════════════
//  Module : src/boss.rs
//  Machine à état : Entering → Flexing → Active(0) → Dying → Dead
//  Patterns : patrol (sinusoïde 5s) → charge (fonce sur le joueur, fin au mur)
//  Particularités :
//    - Intro en spirale (7s) + flexing (1.7s)
//    - Musique dédiée (boss.ogg)
//    - Mouvement patrol sinusoïdal entre les charges
//    - Animation de flexing accéléré pendant la mort

pub static BOSS_PHASES: [PhaseDef; 2] = [
    PhaseDef {
        health: 100,
        enter_sound: Some("audio/t_go.wav"),
        patterns: &[
            PatternDef {
                name: "patrol",
                duration: 5.0,
            },
            PatternDef {
                name: "charge",
                duration: 0.1,
            },
        ],
    },
    PhaseDef {
        health: 100,
        enter_sound: Some("audio/t_go.wav"),
        patterns: &[
            PatternDef {
                name: "patrol",
                duration: 2.0,
            },
            PatternDef {
                name: "charge",
                duration: 0.1,
            },
        ],
    },
];

pub static BOSS: EnemyDef = EnemyDef {
    name: "Boss",
    radius: 80.0,
    sprite_size: 256.0,
    phases: &BOSS_PHASES,
    death_duration: 4.0,
    death_shake_max: 20.0,
    hit_sound: "audio/asteroid_hit.ogg",
    death_explosion_sound: "audio/boss_explosion.ogg",
};

// ═══════════════════════════════════════════════════════════════════════
//  LISTE COMPLÈTE
// ═══════════════════════════════════════════════════════════════════════

/// Tous les ennemis du jeu, pour référence et itération.
pub static ALL_ENEMIES: &[&EnemyDef] = &[&BOSS];
