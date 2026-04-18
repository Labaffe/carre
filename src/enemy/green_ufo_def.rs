//! Green UFO version data-driven.
//!
//! Comportement :
//! - Phase `rush` : fonce vers la dernière position connue du joueur.
//!   Direction figée à l'entrée dans la phase via `SetRushDirection`.
//! - Phase `idle` : stationnaire, pause avant nouveau rush.
//! - Cycle rush ↔ idle jusqu'à mort.
//!
//! Les collisions et dégâts sont pris en charge par `physic::collision` et
//! `enemy::enemy::projectile_enemy_collision` via le composant `Enemy` attaché.

use std::time::Duration;

use bevy::prelude::*;

use crate::enemy::behaviors::{
    DespawnIfOffscreen, PlaySound, RushMove, SetRushDirection,
};
use crate::enemy::system::{b, seq, EnemyDefinition, Phase, PhaseId, Transition, TransitionTrigger};

const RUSH_SPEED: f32 = 800.0;
const RUSH_DURATION: f32 = 1.5;
const IDLE_DURATION: f32 = 1.0;

pub fn green_ufo_definition() -> EnemyDefinition {
    EnemyDefinition {
        name: "GreenUFO",
        initial_phase: PhaseId("rush"),
        phases: vec![
            // ── Rush ──────────────────────────────────────────────
            Phase {
                id: PhaseId("rush"),
                on_enter: Some(seq(vec![
                    b(SetRushDirection),
                    b(PlaySound {
                        path: "audio/sfx/green_ufo.ogg",
                        volume: 0.8,
                    }),
                ])),
                behavior: seq(vec![
                    b(RushMove { speed: RUSH_SPEED }),
                    b(DespawnIfOffscreen { margin: 100.0 }),
                ]),
                transitions: vec![Transition {
                    trigger: TransitionTrigger::Timer(Duration::from_secs_f32(RUSH_DURATION)),
                    target_phase: PhaseId("idle"),
                    priority: 0,
                }],
            },
            // ── Idle ──────────────────────────────────────────────
            Phase {
                id: PhaseId("idle"),
                on_enter: None,
                // Pas de mouvement — stationnaire.
                behavior: b(crate::enemy::system::Noop),
                transitions: vec![Transition {
                    trigger: TransitionTrigger::Timer(Duration::from_secs_f32(IDLE_DURATION)),
                    target_phase: PhaseId("rush"),
                    priority: 0,
                }],
            },
        ],
    }
}
