//! Registre central — config statique (sprite, radius, sons) de chaque type
//! d'ennemi. Les machines à état vivent dans les fichiers d'ennemis.

use bevy::prelude::Color;

use crate::enemy::enemy::EnemyConfig;

pub struct EnemyData {
    pub name: &'static str,
    pub config: EnemyConfigData,
    pub total_hp: i32,
}

pub struct EnemyConfigData {
    pub radius: f32,
    pub sprite_size: f32,
    pub hit_sound: &'static str,
    pub death_explosion_sound: &'static str,
    pub hit_flash_color: Option<Color>,
}

impl EnemyConfigData {
    pub const fn new(
        radius: f32,
        sprite_size: f32,
        hit_sound: &'static str,
        death_explosion_sound: &'static str,
    ) -> Self {
        Self {
            radius,
            sprite_size,
            hit_sound,
            death_explosion_sound,
            hit_flash_color: None,
        }
    }

    pub fn to_config(&self) -> EnemyConfig {
        EnemyConfig {
            radius: self.radius,
            sprite_size: self.sprite_size,
            hit_sound: self.hit_sound,
            death_explosion_sound: self.death_explosion_sound,
            hit_flash_color: self.hit_flash_color,
        }
    }
}

pub const BOSS: EnemyData = EnemyData {
    name: "Boss",
    config: EnemyConfigData {
        radius: 80.0,
        sprite_size: 256.0,
        hit_sound: "audio/sfx/asteroid_hit.ogg",
        death_explosion_sound: "audio/sfx/boss_explosion.ogg",
        hit_flash_color: None, // défini à Color::rgba(2.5,2.5,2.5,1.0) dans boss.rs
    },
    total_hp: 300,
};

pub const GREEN_UFO: EnemyData = EnemyData {
    name: "GreenUFO",
    config: EnemyConfigData::new(
        30.0,
        64.0,
        "audio/sfx/asteroid_hit.ogg",
        "audio/sfx/asteroid_die.ogg",
    ),
    total_hp: 5,
};

pub const ASTEROID: EnemyData = EnemyData {
    name: "Asteroid",
    config: EnemyConfigData::new(
        30.0,
        64.0,
        "audio/sfx/asteroid_hit.ogg",
        "audio/sfx/asteroid_die.ogg",
    ),
    total_hp: 1,
};
