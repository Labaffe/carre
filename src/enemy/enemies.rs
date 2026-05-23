//! Registre central — config statique (sprite, radius) de chaque type
//! d'ennemi. Les machines à état vivent dans les fichiers d'ennemis.

#[derive(Clone, Copy)]
pub struct EnemyData {
    pub name: &'static str,
    pub config: EnemyConfigData,
    pub total_hp: i32,
}

#[derive(Clone, Copy)]
pub struct EnemyConfigData {
    pub radius: f32,
    pub sprite_size: f32,
}

impl EnemyConfigData {
    pub const fn new(radius: f32, sprite_size: f32) -> Self {
        Self {
            radius,
            sprite_size,
        }
    }
}

pub const BOSS: EnemyData = EnemyData {
    name: "Boss",
    config: EnemyConfigData::new(80.0, 256.0),
    total_hp: 150,
};

pub const GREEN_UFO: EnemyData = EnemyData {
    name: "GreenUFO",
    config: EnemyConfigData::new(30.0, 64.0),
    total_hp: 5,
};

pub const ASTEROID: EnemyData = EnemyData {
    name: "Asteroid",
    config: EnemyConfigData::new(30.0, 64.0),
    total_hp: 1,
};

pub const MINE: EnemyData = EnemyData {
    name: "Mine",
    config: EnemyConfigData::new(40.0, 160.0),
    total_hp: 1,
};

pub const KAMIKAZE: EnemyData = EnemyData {
    name: "Kamikaze",
    config: EnemyConfigData::new(18.0, 90.0),
    total_hp: 2,
};

pub const OCTOPUS: EnemyData = EnemyData {
    name: "Octopus",
    config: EnemyConfigData::new(75.0, 192.0),
    total_hp: 70,
};

/// Variante verte de l'octopus — mêmes stats de base que `OCTOPUS` (taille,
/// hitbox, HP) ; les différences vivent dans le builder et les systèmes
/// `octopus_green_*` (cf. `octopus.rs`) : rush aléatoire + intangible,
/// tir éventail à 4 projectiles verts.
pub const OCTOPUS_GREEN: EnemyData = EnemyData {
    name: "OctopusGreen",
    config: EnemyConfigData::new(75.0, 192.0),
    total_hp: 70,
};

/// Tourelle gatling — ennemi statique qui vise le joueur et tire à cadence
/// régulière. Conçu pour être utilisé standalone OU comme enfant d'un futur
/// groupe (vaisseau, etc.) — dans ce dernier cas, sa position locale est
/// héritée du parent via la hiérarchie Bevy.
pub const TURRET: EnemyData = EnemyData {
    name: "Turret",
    config: EnemyConfigData::new(48.0, 140.0),
    total_hp: 30,
};
