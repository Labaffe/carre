//! Gatling version data-driven — **squelette**.
//!
//! La Gatling en jeu existe sous deux formes très différentes :
//! - **attachée au mothership** : position calquée sur le parent via
//!   `MothershipLink`, tire en `aim_and_shoot` ou `full_auto`
//! - **standalone** : entre à l'écran seule depuis le haut
//!
//! Les deux variantes partagent un cycle de phases similaire. On les modélise
//! ici comme DEUX `EnemyDefinition` distinctes (avec les mêmes ids de phases
//! pour la réutilisation des behaviors).
//!
//! ## Points non triviaux pour une migration complète
//! 1. **Ciblage du joueur avec cône + balayage** : il faut un behavior
//!    `AimWithSweep` dédié qui lit le composant `GatlingAimBias` (centre + phase)
//!    et écrit la rotation du sprite.
//! 2. **Tir une fois par pattern `aim_and_shoot`** : le tir doit se faire à
//!    ~70% de la durée du pattern. La phase `aim_and_shoot` dure N secondes
//!    avec un `Timer(0.7N)` qui déclenche une transition vers une sous-phase
//!    `fire_and_recover`.
//! 3. **Full-auto avec ramp-up** : cadence croissante. Nécessite un behavior
//!    qui stocke le rampStart dans un composant ad-hoc.
//! 4. **Laser de visée** : géré séparément par `gatling_laser_update` dans
//!    l'ancien `gatling.rs`. À migrer en behavior `DrawAimLaser` si souhaité.
//!
//! Pour l'instant, on fournit une squelette de définition — les behaviors
//! `GatlingAimAndShoot` et `GatlingFullAuto` doivent être implémentés dans
//! `behaviors.rs` (ou un module dédié `gatling_behaviors.rs`) en lisant les
//! composants `GatlingStyleComp` / `GatlingAimBias` / `GatlingBaseEdge`.

use std::time::Duration;

use crate::enemy::behaviors::PlaySound;
use crate::enemy::system::{b, EnemyDefinition, Noop, Phase, PhaseId, Transition, TransitionTrigger};

/// Gatling standalone (apparait seule, sans parent).
pub fn gatling_standalone_definition() -> EnemyDefinition {
    EnemyDefinition {
        name: "GatlingStandalone",
        initial_phase: PhaseId("entering"),
        phases: vec![
            Phase {
                id: PhaseId("entering"),
                on_enter: Some(b(PlaySound {
                    path: "audio/sfx/gatling_land.ogg",
                    volume: 0.7,
                })),
                // TODO : behavior `GatlingEnteringAnim` (cycle frames + descente verticale)
                behavior: b(Noop),
                transitions: vec![Transition {
                    trigger: TransitionTrigger::Timer(Duration::from_secs_f32(3.5)),
                    target_phase: PhaseId("active"),
                    priority: 0,
                }],
            },
            Phase {
                id: PhaseId("active"),
                on_enter: None,
                // TODO : behavior `GatlingAimAndShoot` ou `GatlingFullAuto`
                // selon le pattern choisi. Pour l'instant Noop.
                behavior: b(Noop),
                transitions: vec![],
            },
        ],
    }
}

/// Gatling montée sur mothership. L'`EnemyBehavior` ne gère pas la position :
/// elle est synchronisée par `mothership_sync_positions` via `MothershipLink`.
///
/// Variante `aim_and_shoot` : suit le joueur et tire à intervalle régulier.
pub fn gatling_turret_aim_definition() -> EnemyDefinition {
    EnemyDefinition {
        name: "GatlingTurretAim",
        initial_phase: PhaseId("entering"),
        phases: vec![
            Phase {
                id: PhaseId("entering"),
                on_enter: None,
                // Animation d'apparition (cycle de frames), position gérée par mothership.
                behavior: b(Noop),
                transitions: vec![Transition {
                    trigger: TransitionTrigger::Timer(Duration::from_secs_f32(3.5)),
                    target_phase: PhaseId("aim_shoot"),
                    priority: 0,
                }],
            },
            Phase {
                id: PhaseId("aim_shoot"),
                on_enter: None,
                // TODO : `seq([AimAtPlayerInCone, PeriodicShoot { interval: 2.0, aim: Forward }])`
                behavior: b(Noop),
                transitions: vec![],
            },
        ],
    }
}

/// Variante `full_auto` : balayage + cadence croissante.
pub fn gatling_turret_fullauto_definition() -> EnemyDefinition {
    EnemyDefinition {
        name: "GatlingTurretFullAuto",
        initial_phase: PhaseId("entering"),
        phases: vec![
            Phase {
                id: PhaseId("entering"),
                on_enter: None,
                behavior: b(Noop),
                transitions: vec![Transition {
                    trigger: TransitionTrigger::Timer(Duration::from_secs_f32(3.5)),
                    target_phase: PhaseId("sweep_shoot"),
                    priority: 0,
                }],
            },
            Phase {
                id: PhaseId("sweep_shoot"),
                on_enter: None,
                // TODO : `seq([SweepAim, PeriodicShoot { interval: ramp, aim: Forward }])`
                behavior: b(Noop),
                transitions: vec![],
            },
        ],
    }
}
