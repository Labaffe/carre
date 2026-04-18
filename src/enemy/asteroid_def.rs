//! Astéroïde version data-driven.
//!
//! Comportement : tombe vers le bas à vitesse constante, despawné hors écran.
//! La difficulté applique un multiplicateur global à la vitesse via la ressource
//! `Difficulty.factor` — le behavior `FallDown` lit directement cette vitesse
//! depuis le composant Transform initial (pas de dynamique frame-par-frame ici
//! car l'astéroïde historique utilise aussi une vitesse fixée au spawn).

use bevy::prelude::*;

use crate::enemy::behaviors::{DespawnIfOffscreen, FallDown};
use crate::enemy::system::{b, seq, EnemyDefinition, Phase, PhaseId};

pub fn asteroid_definition(fall_speed: f32) -> EnemyDefinition {
    EnemyDefinition {
        name: "Asteroid",
        initial_phase: PhaseId("falling"),
        phases: vec![Phase {
            id: PhaseId("falling"),
            on_enter: None,
            behavior: seq(vec![
                b(FallDown { speed: fall_speed }),
                b(DespawnIfOffscreen { margin: 200.0 }),
            ]),
            transitions: vec![],
        }],
    }
}
