//! Arènes : configurations de murs (`Wall`) qui se changent toutes les
//! `WAVES_PER_ARENA` vagues dans le mode Vagues. À chaque rotation, les
//! anciens murs sont despawn et un nouveau pattern est choisi aléatoirement.
//!
//! Patterns conçus pour des écrans 16:9 — positions en coordonnées monde
//! centrées sur (0, 0). Aire de jeu standard ±960 × ±540 (1920p).

use crate::level::waves::WavesConfig;
use crate::physic::wall::{wall_bundle, Wall};
use bevy::prelude::*;

/// Nombre de vagues entre 2 changements d'arène.
const WAVES_PER_ARENA: u32 = 3;

/// Configuration d'un set de murs. Chaque entrée = `(position, size)`.
pub struct ArenaPattern {
    pub name: &'static str,
    pub walls: &'static [(Vec2, Vec2)],
}

/// Catalogue des arènes. Au moins une "vide" pour laisser respirer le joueur
/// périodiquement entre les configurations contraignantes.
const ARENA_PATTERNS: &[ArenaPattern] = &[
    ArenaPattern {
        name: "Vide",
        walls: &[],
    },
    ArenaPattern {
        name: "Murs latéraux",
        walls: &[
            (Vec2::new(-560.0, 0.0), Vec2::new(80.0, 380.0)),
            (Vec2::new(560.0, 0.0), Vec2::new(80.0, 380.0)),
        ],
    },
    ArenaPattern {
        name: "Couloir central",
        walls: &[
            (Vec2::new(0.0, 80.0), Vec2::new(240.0, 90.0)),
            (Vec2::new(0.0, -120.0), Vec2::new(240.0, 90.0)),
        ],
    },
    ArenaPattern {
        name: "Trois piliers",
        walls: &[
            (Vec2::new(-420.0, 50.0), Vec2::new(80.0, 220.0)),
            (Vec2::new(0.0, 50.0), Vec2::new(80.0, 220.0)),
            (Vec2::new(420.0, 50.0), Vec2::new(80.0, 220.0)),
        ],
    },
    ArenaPattern {
        name: "Bunkers",
        walls: &[
            (Vec2::new(-500.0, 220.0), Vec2::new(110.0, 110.0)),
            (Vec2::new(500.0, 220.0), Vec2::new(110.0, 110.0)),
            (Vec2::new(-500.0, -200.0), Vec2::new(110.0, 110.0)),
            (Vec2::new(500.0, -200.0), Vec2::new(110.0, 110.0)),
        ],
    },
    ArenaPattern {
        name: "Croix",
        walls: &[
            (Vec2::new(0.0, 200.0), Vec2::new(80.0, 80.0)),
            (Vec2::new(0.0, -200.0), Vec2::new(80.0, 80.0)),
            (Vec2::new(-220.0, 0.0), Vec2::new(80.0, 80.0)),
            (Vec2::new(220.0, 0.0), Vec2::new(80.0, 80.0)),
        ],
    },
    ArenaPattern {
        name: "Zigzag",
        walls: &[
            (Vec2::new(-350.0, 200.0), Vec2::new(280.0, 70.0)),
            (Vec2::new(350.0, 0.0), Vec2::new(280.0, 70.0)),
            (Vec2::new(-350.0, -200.0), Vec2::new(280.0, 70.0)),
        ],
    },
    ArenaPattern {
        name: "Forteresse haute",
        walls: &[
            (Vec2::new(0.0, 250.0), Vec2::new(400.0, 80.0)),
            (Vec2::new(-450.0, 150.0), Vec2::new(80.0, 200.0)),
            (Vec2::new(450.0, 150.0), Vec2::new(80.0, 200.0)),
        ],
    },
];

/// Tracking de l'arène active. `last_changed_wave = 0` initialement →
/// premier spawn dès la vague 1.
#[derive(Resource, Default)]
pub struct ArenaState {
    pub last_changed_wave: u32,
    pub current_pattern: Option<&'static str>,
}

/// Choisit aléatoirement un nouveau pattern (différent de l'actuel si
/// possible) toutes les `WAVES_PER_ARENA` vagues. Despawn les anciens murs,
/// spawn les nouveaux.
pub fn update_arena(
    waves: Res<WavesConfig>,
    mut arena: ResMut<ArenaState>,
    mut commands: Commands,
    walls_q: Query<Entity, With<Wall>>,
) {
    let n = waves.wave_number;
    if n == 0 {
        return;
    }
    // 1ère arène (last=0 → on n'a encore rien spawn), OU toutes les WAVES_PER_ARENA
    // vagues depuis le dernier change.
    let should_change = arena.last_changed_wave == 0
        || n.saturating_sub(arena.last_changed_wave) >= WAVES_PER_ARENA;
    if !should_change {
        return;
    }

    // Despawn anciens murs.
    for e in walls_q.iter() {
        if let Ok(mut ec) = commands.get_entity(e) {
            ec.try_despawn();
        }
    }

    // Pioche un pattern différent de l'actuel si on a le choix.
    let len = ARENA_PATTERNS.len();
    let mut idx = fastrand::usize(..len);
    if len > 1 {
        if let Some(current_name) = arena.current_pattern {
            // Tentative simple : si on tombe sur le même, on prend le suivant.
            if ARENA_PATTERNS[idx].name == current_name {
                idx = (idx + 1) % len;
            }
        }
    }
    let pattern = &ARENA_PATTERNS[idx];

    // Spawn nouveau pattern.
    for (pos, size) in pattern.walls {
        commands.spawn(wall_bundle(*pos, *size));
    }

    arena.last_changed_wave = n;
    arena.current_pattern = Some(pattern.name);
}

/// Helper : retire la `Resource` ArenaState et despawn tous les murs (appelé
/// en cleanup level si nécessaire). Les murs ont `#[require(GameplayEntity)]`
/// donc le cleanup central de Playing les retire déjà — cette fn est juste
/// pour réinitialiser proprement la Resource.
pub fn reset_arena(mut arena: ResMut<ArenaState>) {
    *arena = ArenaState::default();
}
