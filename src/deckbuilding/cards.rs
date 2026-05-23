//! Cartes du deckbuilding.
//!
//! Chaque `Card` porte un `CardEffect` qui décrit ce qu'elle fait quand
//! elle est sélectionnée par le joueur (lors d'un level-up de score, etc.).
//! L'application de l'effet est faite par le système consommateur (cf.
//! `game_manager::level_up`), pas par ce module.

use crate::player::power::PowerKind;
use crate::weapon::weapon::WeaponKind;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CardType {
    Primary,
    Secondary,
    Passive,
}
impl CardType {
    pub fn to_string(&self) -> String {
        match self {
            CardType::Primary => "Primary".to_string(),
            CardType::Secondary => "Secondary".to_string(),
            CardType::Passive => "Passive".to_string(),
        }
    }
}

/// Effet appliqué au joueur quand la carte est choisie.
#[derive(Clone, Copy, Debug)]
pub enum CardEffect {
    /// Multiplie la vitesse du joueur par `factor` (> 1 = plus rapide).
    SpeedBoost { factor: f32 },
    /// Multiplie la durée d'inter-tir par `factor` (< 1 = cadence plus rapide).
    RapidFire { factor: f32 },
    /// Augmente le cap d'armure du joueur (`PLAYER_MAX_ARMOR` initial).
    ReinforcedArmor { add: u32 },
    /// Remplace l'arme équipée.
    SwapWeapon(WeaponKind),
    /// Remplace le pouvoir équipé (touche Espace).
    SwapPower(PowerKind),
}

#[derive(Clone)]
pub struct Card {
    pub card_type: CardType,
    pub name: String,
    pub requirement: i32,
    pub description: String,
    pub effect: CardEffect,
}

/// Pool de cartes réelles. Le système de level-up pioche 3 cartes
/// aléatoires distinctes parmi celles-ci à chaque palier de score franchi.
pub fn card_pool() -> Vec<Card> {
    vec![
        Card {
            card_type: CardType::Passive,
            name: "Speed Boost".to_string(),
            requirement: 1,
            description: "+20% vitesse de déplacement".to_string(),
            effect: CardEffect::SpeedBoost { factor: 1.2 },
        },
        Card {
            card_type: CardType::Passive,
            name: "Rapid Fire".to_string(),
            requirement: 1,
            description: "+15% cadence de tir".to_string(),
            effect: CardEffect::RapidFire { factor: 0.85 },
        },
        Card {
            card_type: CardType::Passive,
            name: "Reinforced Armor".to_string(),
            requirement: 1,
            description: "+1 armure max".to_string(),
            effect: CardEffect::ReinforcedArmor { add: 1 },
        },
        Card {
            card_type: CardType::Primary,
            name: "Red Cannon".to_string(),
            requirement: 1,
            description: "Tir rouge en éventail".to_string(),
            effect: CardEffect::SwapWeapon(WeaponKind::RedProjectile),
        },
        Card {
            card_type: CardType::Primary,
            name: "Standard Missile".to_string(),
            requirement: 1,
            description: "Missile centré perçant".to_string(),
            effect: CardEffect::SwapWeapon(WeaponKind::StandardMissile),
        },
        Card {
            card_type: CardType::Primary,
            name: "Blue Spread".to_string(),
            requirement: 1,
            description: "5 tirs bleus en éventail".to_string(),
            effect: CardEffect::SwapWeapon(WeaponKind::BlueProjectile),
        },
        Card {
            card_type: CardType::Secondary,
            name: "Shield".to_string(),
            requirement: 1,
            description: "Bouclier sur Espace".to_string(),
            effect: CardEffect::SwapPower(PowerKind::Shield),
        },
        Card {
            card_type: CardType::Secondary,
            name: "Dash".to_string(),
            requirement: 1,
            description: "Dash sur Espace".to_string(),
            effect: CardEffect::SwapPower(PowerKind::Dash),
        },
    ]
}

impl Card {
    /// Pioche une carte aléatoire du pool. Conservé pour compat avec le
    /// système `card_deck::spawn_cards` (touche U) qui pré-remplit 30 cartes.
    pub fn new() -> Card {
        let pool = card_pool();
        let i = fastrand::usize(..pool.len());
        pool[i].clone()
    }
}
