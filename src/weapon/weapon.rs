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
//!
//! Une mini-UI affiche l'arme courante (case avec le sprite du projectile +
//! nom à côté). Mise à jour chaque frame depuis le composant `Weapon` du
//! joueur — si l'arme est swap par une carte deckbuilding, l'UI suit auto.

use crate::game_manager::state::GameState;
use crate::geometry::shape::Shape;
use crate::player::player::Player;
use bevy::prelude::*;

pub struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), setup_weapon_ui)
            .add_systems(OnExit(GameState::Playing), cleanup_weapon_ui)
            .add_systems(
                Update,
                update_weapon_ui.run_if(in_state(GameState::Playing)),
            );
    }
}

// ─── UI de l'arme ────────────────────────────────────────────────────

/// Taille de la case d'icône (px).
const WEAPON_ICON_SIZE: f32 = 48.0;
/// Padding interne de la case (px) — l'image fait `SIZE - PADDING` pour
/// laisser une bordure visible autour du sprite.
const WEAPON_ICON_PADDING: f32 = 8.0;

#[derive(Component)]
struct WeaponUI;
#[derive(Component)]
struct WeaponIcon;
#[derive(Component)]
struct WeaponNameText;

fn setup_weapon_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/PressStart2P-Regular.ttf");
    // Sprite initial = arme par défaut du joueur ; `update_weapon_ui`
    // remplacera si le joueur démarre avec autre chose.
    // Image initiale = arme par défaut (cf. `Weapon::default`).
    let default_kind = Weapon::default().0;
    let initial_image = asset_server.load(default_kind.texture_path());

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                // Sous le bloc bombes (top:92 + ~85px de bombes/hint).
                top: Val::Px(190.0),
                left: Val::Px(20.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                ..default()
            },
            WeaponUI,
        ))
        .with_children(|parent| {
            // Case : fond sombre + sprite centré
            parent
                .spawn((
                    Node {
                        width: Val::Px(WEAPON_ICON_SIZE),
                        height: Val::Px(WEAPON_ICON_SIZE),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.6)),
                ))
                .with_children(|cell| {
                    cell.spawn((
                        ImageNode::new(initial_image),
                        Node {
                            width: Val::Px(WEAPON_ICON_SIZE - WEAPON_ICON_PADDING),
                            height: Val::Px(WEAPON_ICON_SIZE - WEAPON_ICON_PADDING),
                            ..default()
                        },
                        WeaponIcon,
                    ));
                });
            // Nom de l'arme à droite de la case
            parent.spawn((
                Text::new(default_kind.name()),
                TextFont {
                    font,
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                WeaponNameText,
            ));
        });
}

/// Met à jour l'icône et le nom selon l'arme courante du joueur. Tolère
/// l'absence de joueur (entre la sortie du level et le respawn).
fn update_weapon_ui(
    asset_server: Res<AssetServer>,
    weapon_q: Query<&Weapon, With<Player>>,
    mut icon_q: Query<&mut ImageNode, With<WeaponIcon>>,
    mut text_q: Query<&mut Text, With<WeaponNameText>>,
) {
    let Ok(weapon) = weapon_q.single() else { return };

    if let Ok(mut icon) = icon_q.single_mut() {
        // `asset_server.load` est idempotent : même path → même handle (cache).
        icon.image = asset_server.load(weapon.0.texture_path());
    }
    if let Ok(mut text) = text_q.single_mut() {
        let new_name = weapon.0.name();
        if **text != new_name {
            **text = new_name.to_string();
        }
    }
}

fn cleanup_weapon_ui(mut commands: Commands, q: Query<Entity, With<WeaponUI>>) {
    for entity in q.iter() {
        if let Ok(mut e) = commands.get_entity(entity) {
            e.try_despawn();
        }
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

// ─── WeaponKind enum (palette swappable) ──────────────────────────

/// Enum centralisant toutes les armes disponibles. Le composant `Weapon`
/// porte une variante de cet enum sur le joueur ; le swap se fait par
/// `weapon.0 = WeaponKind::X;` (atomique).
///
/// Ajouter une arme : nouvelle variante + nouveau bras dans `def()` (le
/// compilateur force l'exhaustivité — pas de chemin oublié).
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum WeaponKind {
    StandardMissile,
    RedProjectile,
    BlueProjectile,
}

impl WeaponKind {
    /// Liste exhaustive (pour énumération côté deckbuilding/menu).
    pub const ALL: &'static [WeaponKind] = &[
        Self::StandardMissile,
        Self::RedProjectile,
        Self::BlueProjectile,
    ];

    /// Renvoie la `WeaponDef` (stats complètes) de cette arme.
    pub fn def(&self) -> WeaponDef {
        match self {
            Self::StandardMissile => STANDARD_MISSILE,
            Self::RedProjectile => RED_PROJECTILE,
            Self::BlueProjectile => BLUE_PROJECTILE,
        }
    }

    pub fn name(&self) -> &'static str {
        self.def().name
    }

    pub fn texture_path(&self) -> &'static str {
        self.def().texture_path
    }
}

// ─── Composant ───────────────────────────────────────────────────────

/// Composant attaché au joueur qui indique son arme actuelle.
/// Pour swap depuis le deckbuilding : `weapon.0 = WeaponKind::X;`.
#[derive(Component, Clone, Copy)]
pub struct Weapon(pub WeaponKind);

impl Default for Weapon {
    fn default() -> Self {
        Self(WeaponKind::RedProjectile)
    }
}
