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
//! Pour ajouter une arme : créer un `const WeaponDef` et l'assigner dans `update_player_weapon`.
//! Le joueur passe automatiquement de Standard Missile à Red Projectile après 10 secondes.

use bevy::prelude::*;
use crate::difficulty::Difficulty;
use crate::player::Player;
use crate::state::GameState;

pub struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            update_player_weapon.run_if(in_state(GameState::Playing)),
        );
    }
}

// ─── Hitbox ──────────────────────────────────────────────────────────

/// Forme de la hitbox d'un projectile.
#[derive(Clone)]
pub enum HitboxShape {
    /// Cercle simple (rayon).
    Circle(f32),
    /// Rectangle orienté (demi-longueur dans l'axe, demi-largeur perpendiculaire).
    Rect { half_length: f32, half_width: f32 },
}

// ─── Pattern de tir ──────────────────────────────────────────────────

/// Un projectile dans le pattern : angle relatif (en radians) par rapport à la visée.
/// 0.0 = droit devant, positif = gauche, négatif = droite.
#[derive(Clone)]
pub struct ShotAngle(pub f32);

// ─── WeaponDef ───────────────────────────────────────────────────────

/// Définition complète d'une arme.
/// Pour créer une nouvelle arme il suffit de définir une `const WeaponDef`.
#[derive(Clone)]
pub struct WeaponDef {
    pub name: &'static str,
    pub texture_path: &'static str,
    pub hitbox: HitboxShape,
    /// Vitesse des projectiles (px/s).
    pub speed: f32,
    /// Intervalle entre deux tirs (secondes).
    pub fire_rate: f32,
    /// Pattern de tir : liste d'angles relatifs.
    /// Un seul élément `[ShotAngle(0.0)]` = tir simple droit devant.
    /// Trois éléments = éventail style fusil à pompe.
    pub pattern: &'static [ShotAngle],
    /// Dossier optionnel contenant les frames de mort du projectile.
    /// Ex: "images/projectiles/death_missile/" avec frame_000.png, frame_001.png…
    /// Si `None`, le projectile disparaît sans animation.
    pub death_folder: Option<&'static str>,
}

// ─── Armes ───────────────────────────────────────────────────────────

pub const STANDARD_MISSILE: WeaponDef = WeaponDef {
    name: "Standard Missile",
    texture_path: "images/projectiles/missile.png",
    hitbox: HitboxShape::Circle(6.0),
    speed: 900.0,
    fire_rate: 0.2,
    pattern: &[ShotAngle(0.0)], // tir unique droit devant
    death_folder: None,         // disparaît sans animation
};

pub const RED_PROJECTILE: WeaponDef = WeaponDef {
    name: "Red Projectile",
    texture_path: "images/projectiles/red_projectile.png",
    hitbox: HitboxShape::Rect { half_length: 32.0, half_width: 4.0 },
    speed: 1100.0,
    fire_rate: 0.15,
    pattern: &[                 // éventail fusil à pompe
        ShotAngle(0.0),         //   central
        ShotAngle(0.18),        //   gauche (~10°)
        ShotAngle(-0.18),       //   droite (~10°)
    ],
    death_folder: None,         // pas d'animation de mort pour l'instant
};

pub const BLUE_PROJECTILE: WeaponDef = WeaponDef {
    name: "Blue Projectiles",
    texture_path: "images/projectiles/blue_projectile.png",
    hitbox: HitboxShape::Rect { half_length: 32.0, half_width: 4.0 },
    speed: 3300.0, // vitesse triplée par rapport à Red Projectile
    fire_rate: 0.15,
    pattern: &[
        ShotAngle(0.0),         //   central
        ShotAngle(0.12),        //   légèrement gauche
        ShotAngle(-0.12),       //   légèrement droite
        ShotAngle(0.24),        //   gauche (~14°)
        ShotAngle(-0.24),       //   droite (~14°)
    ],
    death_folder: None,
};

// ─── Composant ───────────────────────────────────────────────────────

/// Composant attaché au joueur qui indique son arme actuelle.
#[derive(Component, Clone)]
pub struct Weapon {
    pub def: WeaponDef,
}

impl Default for Weapon {
    fn default() -> Self {
        Self { def: STANDARD_MISSILE }
    }
}

// ─── Système ─────────────────────────────────────────────────────────

/// Met à jour l'arme du joueur selon la phase.
fn update_player_weapon(
    difficulty: Res<Difficulty>,
    mut query: Query<&mut Weapon, With<Player>>,
) {
    let boss_rotation_active = match difficulty.boss_music_start_time {
        Some(start) => difficulty.elapsed >= start + 3.0,
        None => false,
    };

    for mut weapon in query.iter_mut() {
        if boss_rotation_active && weapon.def.name != BLUE_PROJECTILE.name {
            weapon.def = BLUE_PROJECTILE;
        } else if !boss_rotation_active && difficulty.elapsed >= 10.0 && weapon.def.name != RED_PROJECTILE.name {
            weapon.def = RED_PROJECTILE;
        }
    }
}
