//! Mothership Heart version data-driven.
//!
//! Le cœur du mothership est une entité quasi-passive : elle reste collée à son
//! parent (le mothership) et devient vulnérable seulement quand la dernière
//! gatling est détruite (phase 2 du mothership). Ici on ne modélise que la
//! phase `idle` — la destruction et le lien au parent restent gérés par
//! `enemy::mothership` (attachement via `MothershipLink`) pour l'instant, car
//! le parentage nécessite un systemique plus profond que cette migration.

use crate::enemy::system::{b, EnemyDefinition, Noop, Phase, PhaseId};

pub fn mothership_heart_definition() -> EnemyDefinition {
    EnemyDefinition {
        name: "MothershipHeart",
        initial_phase: PhaseId("idle"),
        phases: vec![Phase {
            id: PhaseId("idle"),
            on_enter: None,
            // Le mouvement est piloté par le système `mothership_sync_positions`
            // qui replace ce heart à chaque frame selon la position de son parent
            // (via le composant `MothershipLink`).
            behavior: b(Noop),
            transitions: vec![],
        }],
    }
}
