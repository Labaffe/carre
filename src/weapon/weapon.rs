//! Système d'armes du joueur.
//!
//! Chaque arme est une `WeaponDef` (const) qui définit :
//! - `texture_path` : sprite du projectile
//! - `hitbox`       : Circle(rayon) ou Rect { half_length, half_width }
//! - `speed`        : vitesse en px/s
//! - `fire_rate`    : intervalle entre deux tirs (secondes)
//! - `pattern`      : liste de ShotAngle (angles relatifs en radians, 0 = droit devant)
//! - `death_folder` : dossier optionnel de frames de mort du projectile
//!
//! Le joueur démarre avec `RED_PROJECTILE` (default `Weapon`). Le deckbuilding
//! pourra ultérieurement swap `weapon.def` via une carte. Les autres
//! constantes (`STANDARD_MISSILE`, `BLUE_PROJECTILE`) restent disponibles
//! comme palette d'armes ré-assignables.

use crate::geometry::shape::Shape;
use bevy::prelude::*;

pub struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, _app: &mut App) {
        // Plugin gardé comme hook : le deckbuilding y branchera ses systèmes
        // de modification d'arme quand il sera prêt.
    }
}

// ─── Pattern de tir ──────────────────────────────────────────────────

/// Un projectile dans le pattern : angle relatif (en radians) par rapport à la visée.
/// 0.0 = droit devant, positif = gauche, négatif = droite.
#[derive(Clone)]
pub struct ShotAngle(pub f32);

// ─── WeaponDef ───────────────────────────────────────────────────────

/// Définition complète d'une arme.
#[derive(Clone)]
pub struct WeaponDef {
    pub name: &'static str,
    pub texture_path: &'static str,
    pub hitbox: Shape,
    /// Vitesse des projectiles (px/s).
    pub speed: f32,
    /// Intervalle entre deux tirs (secondes).
    pub fire_rate: f32,
    /// Pattern de tir : liste d'angles relatifs.
    pub pattern: &'static [ShotAngle],
    /// Dossier optionnel contenant les frames de mort du projectile.
    pub death_folder: Option<&'static str>,
}

// ─── Palette d'armes ─────────────────────────────────────────────────

pub const STANDARD_MISSILE: WeaponDef = WeaponDef {
    name: "Standard Missile",
    texture_path: "images/projectiles/missile.png",
    hitbox: Shape::Circle(6.0),
    speed: 900.0,
    fire_rate: 0.2,
    pattern: &[ShotAngle(0.0)],
    death_folder: None,
};

pub const RED_PROJECTILE: WeaponDef = WeaponDef {
    name: "Red Projectile",
    texture_path: "images/projectiles/red_projectile.png",
    hitbox: Shape::Rect {
        half_length: 32.0,
        half_width: 4.0,
    },
    speed: 1100.0,
    fire_rate: 0.15,
    pattern: &[
        ShotAngle(0.0),
        ShotAngle(0.18),
        ShotAngle(-0.18),
    ],
    death_folder: None,
};

pub const BLUE_PROJECTILE: WeaponDef = WeaponDef {
    name: "Blue Projectiles",
    texture_path: "images/projectiles/blue_projectile.png",
    hitbox: Shape::Rect {
        half_length: 32.0,
        half_width: 4.0,
    },
    speed: 3300.0,
    fire_rate: 0.15,
    pattern: &[
        ShotAngle(0.0),
        ShotAngle(0.12),
        ShotAngle(-0.12),
        ShotAngle(0.24),
        ShotAngle(-0.24),
    ],
    death_folder: None,
};

// ─── Composant ───────────────────────────────────────────────────────

/// Composant attaché au joueur qui indique son arme actuelle. `def` peut
/// être swappé à la volée (ex: via le deckbuilding).
#[derive(Component, Clone)]
pub struct Weapon {
    pub def: WeaponDef,
}

impl Default for Weapon {
    fn default() -> Self {
        Self {
            def: RED_PROJECTILE,
        }
    }
}
