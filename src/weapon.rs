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

/// Définition d'une arme : sprite, hitbox, vitesse, cadence, nombre de projectiles.
#[derive(Clone)]
pub struct WeaponDef {
    pub name: &'static str,
    pub texture_path: &'static str,
    /// Demi-longueur de la hitbox (dans l'axe du missile).
    pub hitbox_half_length: f32,
    /// Demi-largeur de la hitbox (perpendiculaire à l'axe).
    pub hitbox_half_width: f32,
    pub speed: f32,
    pub fire_rate: f32,
    pub projectile_count: u32,
    pub side_offset: f32,
}

pub const STANDARD_MISSILE: WeaponDef = WeaponDef {
    name: "Standard Missile",
    texture_path: "images/missile.png",
    hitbox_half_length: 6.0,
    hitbox_half_width: 6.0,
    speed: 600.0,
    fire_rate: 0.2,
    projectile_count: 1,
    side_offset: 0.0,
};

pub const RED_PROJECTILE: WeaponDef = WeaponDef {
    name: "Red Projectile",
    texture_path: "images/red_projectile.png",
    hitbox_half_length: 32.0,  // 64px de long
    hitbox_half_width: 4.0,    // étroit comme une fusée
    speed: 750.0,
    fire_rate: 0.15,
    projectile_count: 3,
    side_offset: 20.0,
};

/// Composant attaché au joueur qui indique son arme actuelle.
#[derive(Component, Clone)]
pub struct Weapon {
    pub def: WeaponDef,
}

impl Default for Weapon {
    fn default() -> Self {
        Self {
            def: STANDARD_MISSILE,
        }
    }
}

/// Passe automatiquement à Red Projectile après 10 secondes.
fn update_player_weapon(
    difficulty: Res<Difficulty>,
    mut query: Query<&mut Weapon, With<Player>>,
) {
    for mut weapon in query.iter_mut() {
        if difficulty.elapsed >= 10.0 && weapon.def.name != RED_PROJECTILE.name {
            weapon.def = RED_PROJECTILE;
        }
    }
}
