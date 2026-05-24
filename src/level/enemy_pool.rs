//! Pool d'ennemis partagé entre les niveaux non-scriptés.
//!
//! Centralise la **valeur** de chaque ennemi (cost/weight + kind de spawn).
//! Utilisé par :
//! - [`crate::level::chaos`] — tirages pondérés à budget par tier
//! - [`crate::level::waves`] — vagues homogènes piochées via le même pool
//!
//! Pour ajuster la difficulté globale, modifier `default_tunings()` ici :
//! tout consommateur du pool sera automatiquement aligné.

use crate::game_manager::difficulty::{Difficulty, SpawnPosition};
use bevy::prelude::*;

/// Mode de spawn d'un tuning :
/// - `Single` : pousse une `SpawnRequest` standard dans `difficulty.spawn_requests`
/// - `SimpleUfoWave` : spawn directement un `SimpleUfoWaveSpawner` (path Bézier)
#[derive(Clone, Copy)]
pub enum SpawnKind {
    Single,
    SimpleUfoWave { count: usize, interval: f32 },
}

/// `cost` est consommé sur le budget de la vague/tier au moment du spawn.
/// `weight` pondère la probabilité de sélection parmi les ennemis affordables.
#[derive(Clone)]
pub struct EnemyTuning {
    pub name: &'static str,
    pub cost: u32,
    pub weight: u32,
    pub kind: SpawnKind,
}

/// Source de vérité unique du jeu. Pour rééquilibrer un ennemi → modifier ici.
///
/// - `asteroid` (cost 1, weight 50) : remplisseur ultra-courant
/// - `kamikaze` (cost 5, weight 25) : threat mid, fréquent dès tier 1
/// - `green_ufo` (cost 4) / `mine` (cost 3) : variété
/// - `simple_ufo_shooter` (cost 6) : tireur, mid-tier
/// - `simple_ufo_wave` (cost 8) : wave Bézier de 5 ufos
/// - `octopus` / `octopus_green` (cost 15-18) : mid-late game
/// - `vaisseau` (cost 25) : gros groupe
/// - `boss` (cost 60, weight 1) : très rare, débloqué tier 5+
pub fn default_tunings() -> Vec<EnemyTuning> {
    vec![
        EnemyTuning { name: "asteroid",           cost: 1,  weight: 50, kind: SpawnKind::Single },
        EnemyTuning { name: "kamikaze",           cost: 5,  weight: 25, kind: SpawnKind::Single },
        EnemyTuning { name: "green_ufo",          cost: 4,  weight: 12, kind: SpawnKind::Single },
        EnemyTuning { name: "mine",               cost: 3,  weight: 12, kind: SpawnKind::Single },
        EnemyTuning { name: "simple_ufo_shooter", cost: 6,  weight: 10, kind: SpawnKind::Single },
        EnemyTuning { name: "simple_ufo_wave",    cost: 8,  weight: 8,  kind: SpawnKind::SimpleUfoWave { count: 5, interval: 0.2 } },
        EnemyTuning { name: "octopus",            cost: 15, weight: 5,  kind: SpawnKind::Single },
        EnemyTuning { name: "octopus_green",      cost: 18, weight: 4,  kind: SpawnKind::Single },
        EnemyTuning { name: "vaisseau",           cost: 25, weight: 3,  kind: SpawnKind::Single },
        EnemyTuning { name: "boss",               cost: 60, weight: 1,  kind: SpawnKind::Single },
    ]
}

/// Tirage pondéré d'un ennemi parmi ceux affordables. Renvoie `None` si
/// le budget est inférieur au plus petit cost.
pub fn pick_enemy<'a>(enemies: &'a [EnemyTuning], budget: i32) -> Option<&'a EnemyTuning> {
    let affordable: Vec<&EnemyTuning> = enemies
        .iter()
        .filter(|e| (e.cost as i32) <= budget)
        .collect();
    if affordable.is_empty() {
        return None;
    }
    let total_weight: u32 = affordable.iter().map(|e| e.weight).sum();
    if total_weight == 0 {
        return None;
    }
    let mut roll = fastrand::u32(0..total_weight);
    for e in affordable {
        if roll < e.weight {
            return Some(e);
        }
        roll -= e.weight;
    }
    None
}

/// Dispatch le spawn selon le `kind` : ennemi standard via la file de
/// spawn, ou wave Bézier via un composant `SimpleUfoWaveSpawner` direct.
pub fn spawn_picked(
    commands: &mut Commands,
    difficulty: &mut Difficulty,
    pick: &EnemyTuning,
    spawn_position: SpawnPosition,
) {
    match pick.kind {
        SpawnKind::Single => {
            difficulty.spawn_requests.push((pick.name, 1, spawn_position));
        }
        SpawnKind::SimpleUfoWave { count, interval } => {
            commands.spawn(crate::enemy::simple_ufo::SimpleUfoWaveSpawner::new(
                count, interval,
            ));
        }
    }
}
