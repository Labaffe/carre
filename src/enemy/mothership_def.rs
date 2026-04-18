//! Mothership version data-driven — **squelette**.
//!
//! Le Mothership est un ennemi multi-entité (parent + gatlings + hearts) avec
//! un cycle de vie à 3 phases distinctes :
//! - **entering** : descente + décélération lourde depuis hors-écran (3.5s)
//! - **phase_1** : flottement ample, gatlings vivantes, hearts invulnérables
//! - **phase_2** : transition quand toutes gatlings mortes — dérive recentrée
//!   sur les hearts, ceux-ci deviennent la seule cible
//! - **dying** : shake + explosions en cascade, musique s'arrête
//!
//! ## Points à finaliser pour une migration complète
//! 1. **Spawn des enfants (gatlings + hearts)** en on_enter via un behavior
//!    `SpawnChildren` qui lit une config (positions normalisées).
//! 2. **Détection "toutes gatlings mortes"** : transition `phase_1 → phase_2`
//!    via un `TransitionTrigger::Custom` qui compte les entités `GatlingMarker`
//!    enfants (ou via un event `AllGatlingsDead` émis par un système auxiliaire).
//! 3. **Synchronisation des enfants** : garder le système
//!    `mothership_sync_positions` qui replace les enfants à chaque frame —
//!    c'est un système d'infrastructure qui n'a pas lieu d'être migré en
//!    behavior.
//! 4. **Decor mirror sprites** (6 sprites formant le mothership) : spawned as
//!    children au moment du spawn, pas besoin de behaviors.

use std::time::Duration;

use crate::enemy::behaviors::{DespawnSelf, DriftFloat, EnterFromOffscreen, PlaySound, ShakeAround};
use crate::enemy::system::{
    b, seq, EnemyDefinition, Noop, Phase, PhaseId, Transition, TransitionTrigger,
};
use bevy::prelude::*;

const ENTERING_DURATION: f32 = 3.5;
const PHASE_TRANSITION_DURATION: f32 = 2.0;
const DYING_DURATION: f32 = 4.0;

pub fn mothership_definition(target_pos: Vec3) -> EnemyDefinition {
    EnemyDefinition {
        name: "Mothership",
        initial_phase: PhaseId("entering"),
        phases: vec![
            // ── Entering ─────────────────────────────────────────
            Phase {
                id: PhaseId("entering"),
                on_enter: Some(b(PlaySound {
                    path: "audio/sfx/mothership_land.ogg",
                    volume: 0.8,
                })),
                behavior: b(EnterFromOffscreen {
                    from: target_pos + Vec3::new(-1100.0, 650.0, 0.0),
                    to: target_pos,
                    duration: ENTERING_DURATION,
                }),
                transitions: vec![Transition {
                    trigger: TransitionTrigger::Timer(Duration::from_secs_f32(ENTERING_DURATION)),
                    target_phase: PhaseId("phase_1"),
                    priority: 0,
                }],
            },
            // ── Phase 1 : gatlings actives, flottement ample ─────
            Phase {
                id: PhaseId("phase_1"),
                // TODO on_enter : spawn_children (gatlings + hearts)
                on_enter: None,
                behavior: b(DriftFloat {
                    main_amp_x: 700.0,
                    main_freq_x: 0.4,
                    minor_amp_y: 100.0,
                    minor_freq_y: 0.55,
                    anchor: target_pos,
                }),
                transitions: vec![
                    // Transition déclenchée par un Custom predicate qui
                    // compte les gatlings enfants (ou par un event externe).
                    // Pour l'instant, simple trigger temporel (placeholder).
                    Transition {
                        trigger: TransitionTrigger::Event("all_gatlings_dead"),
                        target_phase: PhaseId("phase_2"),
                        priority: 10,
                    },
                ],
            },
            // ── Phase 2 : gatlings mortes, hearts cibles ─────────
            Phase {
                id: PhaseId("phase_2"),
                on_enter: None,
                behavior: b(DriftFloat {
                    main_amp_x: 700.0,
                    main_freq_x: 0.4,
                    minor_amp_y: 200.0,
                    minor_freq_y: 0.55,
                    anchor: target_pos + Vec3::new(0.0, -600.0, 0.0),
                }),
                transitions: vec![Transition {
                    trigger: TransitionTrigger::Event("all_hearts_dead"),
                    target_phase: PhaseId("dying"),
                    priority: 10,
                }],
            },
            // ── Dying : shake + explosions ───────────────────────
            Phase {
                id: PhaseId("dying"),
                on_enter: Some(seq(vec![
                    b(PlaySound {
                        path: "audio/sfx/boss_explosion.ogg",
                        volume: 1.0,
                    }),
                    // TODO : StopBossMusic (behavior dédié)
                ])),
                behavior: b(ShakeAround { amplitude: 15.0 }),
                transitions: vec![Transition {
                    trigger: TransitionTrigger::Timer(Duration::from_secs_f32(DYING_DURATION)),
                    target_phase: PhaseId("dead"),
                    priority: 0,
                }],
            },
            // ── Dead : despawn ──────────────────────────────────
            Phase {
                id: PhaseId("dead"),
                on_enter: Some(b(DespawnSelf)),
                behavior: b(Noop),
                transitions: vec![],
            },
        ],
    }
}
