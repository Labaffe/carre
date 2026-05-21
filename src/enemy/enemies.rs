//! Registre central — config statique (sprite, radius) de chaque type
//! d'ennemi. Les machines à état vivent dans les fichiers d'ennemis.

pub struct EnemyData {
    pub name: &'static str,
    pub config: EnemyConfigData,
    pub total_hp: i32,
}

pub struct EnemyConfigData {
    pub radius: f32,
    pub sprite_size: f32,
}

impl EnemyConfigData {
    pub const fn new(radius: f32, sprite_size: f32) -> Self {
        Self { radius, sprite_size }
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
    config: EnemyConfigData::new(25.0, 128.0),
    total_hp: 2,
};

pub const OCTOPUS: EnemyData = EnemyData {
    name: "Octopus",
    config: EnemyConfigData::new(50.0, 128.0),
    total_hp: 70,
};
