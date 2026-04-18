//! Registre central de tous les niveaux du jeu.
//!
//! Chaque niveau est défini par un `LevelDef` : sa config visuelle,
//! son déroulement (timeline), et ses paramètres d'intro.
//! Pour ajouter un nouveau niveau :
//! 1. Définir son `LevelDef` ici
//! 2. Ajouter sa fonction `build_level_N()` dans `level.rs`
//! 3. Ajouter une entrée dans `ALL_LEVELS`

/// Direction du scroll du background.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScrollDirection {
    /// Scroll vertical de haut en bas.
    Down,
    /// Scroll vertical de bas en haut.
    Up,
    /// Scroll horizontal de droite à gauche.
    Left,
    /// Scroll horizontal de gauche à droite.
    Right,
}

impl ScrollDirection {
    /// Le scroll est-il horizontal ?
    pub fn is_horizontal(self) -> bool {
        matches!(self, ScrollDirection::Left | ScrollDirection::Right)
    }

    /// Rotation de la tile de background pour cette direction de scroll.
    /// Down/Up → 0° (pas de rotation), Left/Right → 90°.
    pub fn bg_tile_rotation(self) -> f32 {
        match self {
            ScrollDirection::Down | ScrollDirection::Up => 0.0,
            ScrollDirection::Left | ScrollDirection::Right => -std::f32::consts::FRAC_PI_2,
        }
    }
}

/// Définition d'un niveau : config visuelle + paramètres.
pub struct LevelDef {
    /// Nom affiché (menu, debug).
    pub name: &'static str,
    /// Sprite du vaisseau joueur (chemin depuis assets/).
    pub player_ship: &'static str,
    /// Image de la tile de background (chemin depuis assets/).
    pub background_tile: &'static str,
    /// Direction du scroll du background.
    pub scroll_direction: ScrollDirection,
}

// ═══════════════════════════════════════════════════════════════════════
//  Définitions des niveaux
// ═══════════════════════════════════════════════════════════════════════

/// Niveau 1 — Space Invader
/// Scroll vertical (haut → bas), vaisseau pointe vers le haut.
pub const LEVEL_SPACE_INVADER: LevelDef = LevelDef {
    name: "Space Invader",
    player_ship: "images/player_ship/ship_0.png",
    background_tile: "images/backgrounds/space_background_tile.png",
    scroll_direction: ScrollDirection::Down,
};

/// Niveau 2 — MotherShip
/// Scroll horizontal (droite → gauche), vaisseau pointe vers la droite.
pub const LEVEL_MOTHERSHIP: LevelDef = LevelDef {
    name: "MotherShip",
    player_ship: "images/player_ship/ship_1.png",
    background_tile: "images/backgrounds/space_background_tile_2.png",
    scroll_direction: ScrollDirection::Left,
};

/// Liste ordonnée de tous les niveaux (1-indexed via index+1).
pub const ALL_LEVELS: &[&LevelDef] = &[
    &LEVEL_SPACE_INVADER, // Niveau 1
    &LEVEL_MOTHERSHIP,    // Niveau 2
];

/// Retourne la définition d'un niveau (1-indexed).
pub fn level_def(level: usize) -> &'static LevelDef {
    ALL_LEVELS
        .get(level - 1)
        .unwrap_or(&ALL_LEVELS[0])
}

/// Retourne le nom d'un niveau (1-indexed).
pub fn level_name(level: usize) -> &'static str {
    level_def(level).name
}
