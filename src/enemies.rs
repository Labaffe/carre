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

pub static BOSS_PHASES: [PhaseDef; 3] = [
    // Phase 1 : patrol 5s + charge, transition vers phase 2
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
        has_transition: true,
    },
    // Phase 2 : patrol 4s + charge, transition vers phase 3
    PhaseDef {
        health: 100,
        enter_sound: Some("audio/t_go.wav"),
        patterns: &[
            PatternDef {
                name: "patrol",
                duration: 4.0,
            },
            PatternDef {
                name: "charge",
                duration: 0.1,
            },
        ],
        has_transition: true,
    },
    // Phase 3 : patrol 2.5s + charge, pas de transition → mort
    PhaseDef {
        health: 100,
        enter_sound: Some("audio/t_go.wav"),
        patterns: &[
            PatternDef {
                name: "patrol",
                duration: 3.0,
            },
            PatternDef {
                name: "charge",
                duration: 0.1,
            },
        ],
        has_transition: false,
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
//  GREEN UFO
// ═══════════════════════════════════════════════════════════════════════
//  Module : src/green_ufo.rs
//  Machine à état : Active(0) → Dying → Dead  (pas d'intro ni de flexing)
//  Patterns : rush (fonce sur le joueur 0.4s) → idle (pause 0.2s) → repeat
//  Particularités :
//    - Mort instantanée style astéroïde (explosion + despawn)
//    - Son "green_ufo.ogg" à chaque rush
//    - Spawn périodique depuis le haut de l'écran

pub static GREEN_UFO_PHASES: [PhaseDef; 1] = [PhaseDef {
    health: 5,
    enter_sound: None,
    patterns: &[
        PatternDef {
            name: "rush",
            duration: 1.5,
        },
        // c'est pervert mais la durée du "idle" correspond au temps de rush du pattern précédent
        PatternDef {
            name: "idle",
            duration: 1.0,
        },
    ],
    has_transition: false,
}];

pub static GREEN_UFO: EnemyDef = EnemyDef {
    name: "GreenUFO",
    radius: 30.0,
    sprite_size: 64.0,
    phases: &GREEN_UFO_PHASES,
    death_duration: 0.05,
    death_shake_max: 0.0,
    hit_sound: "audio/asteroid_hit.ogg",
    death_explosion_sound: "audio/asteroid_die.ogg",
};

// ═══════════════════════════════════════════════════════════════════════
//  LISTE COMPLÈTE
// ═══════════════════════════════════════════════════════════════════════

/// Tous les ennemis du jeu, pour référence et itération.
pub static ALL_ENEMIES: &[&EnemyDef] = &[&BOSS, &GREEN_UFO];
