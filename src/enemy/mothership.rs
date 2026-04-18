//! Mothership — vaisseau mère portant des Gatlings et des Hearts.
//!
//! Types partagés (config, bord d'entrée, composants), constantes de mouvement,
//! et systèmes du Mothership lui-même (spawn, entering, drift, sync, death, dying).
//!
//! Les systèmes spécifiques aux Gatlings sont dans `gatling.rs`,
//! ceux des Hearts dans `mothership_heart.rs`.

use crate::enemy::enemies::{GATLING, MOTHERSHIP_HEART};
use crate::enemy::enemy::{Enemy, EnemyState, PatternIndex, PatternTimer};
use crate::fx::explosion::load_frames_from_folder;
use crate::game_manager::difficulty::SpawnPosition;
use crate::item::item::{DropTable, ItemType};
use crate::level::level::{Action, LevelActionEvent};
use bevy::prelude::*;

// ═══════════════════════════════════════════════════════════════════════
//  Constantes
// ═══════════════════════════════════════════════════════════════════════

/// Durée totale de l'entrée (secondes). Rapide mais longue décélération.
pub(crate) const MOTHERSHIP_ENTERING_DURATION: f32 = 3.5;
/// Distance verticale parcourue (axe d'entrée, pixels).
const MOTHERSHIP_ENTER_VERTICAL: f32 = 650.0;
/// Décalage latéral au spawn (pixels, négatif = gauche).
const MOTHERSHIP_LATERAL_OFFSET: f32 = -1100.0;
/// Distance latérale parcourue (rattrape le décalage vers le centre).
const MOTHERSHIP_ENTER_LATERAL: f32 = 1100.0;
/// Exposant d'inertie pour l'easing. Plus c'est haut, plus le mothership
/// arrive vite au début et décélère lourdement à la fin (masse énorme).
const MOTHERSHIP_INERTIA_POWER: i32 = 5;

/// Distance nominale pour le calcul du spawn hors-écran.
pub(crate) const MOTHERSHIP_ENTERING_DISTANCE: f32 = 650.0;
/// Taille du sprite Gatling (pixels).
pub(crate) const GATLING_SPRITE_SIZE: f32 = 128.0;
/// Durée totale du recul du Mothership pendant la mort (secondes).
const MOTHERSHIP_DYING_DURATION: f32 = 3.5;

// ─── Flottement Phase 1 (gatlings vivantes) ───────────────────────
/// Amplitude du flottement principal — horizontal (pixels).
const P1_DRIFT_MAIN_AMP: f32 = 700.0;
/// Amplitude du flottement secondaire — vertical, plus ample (pixels).
const P1_DRIFT_MINOR_AMP: f32 = 100.0;
/// Fréquence du flottement principal (rad/s).
const P1_DRIFT_MAIN_FREQ: f32 = 0.4;
/// Fréquence du flottement secondaire (rad/s).
const P1_DRIFT_MINOR_FREQ: f32 = 0.55;

// ─── Flottement Phase 2 (gatlings mortes, hearts restants) ────────
/// Amplitude du flottement principal — horizontal (pixels).
const P2_DRIFT_MAIN_AMP: f32 = 700.0;
/// Amplitude du flottement secondaire — vertical, réduit pour garder les hearts visibles (pixels).
const P2_DRIFT_MINOR_AMP: f32 = 200.0;
/// Fréquence du flottement principal (rad/s).
const P2_DRIFT_MAIN_FREQ: f32 = 0.4;
/// Fréquence du flottement secondaire (rad/s).
const P2_DRIFT_MINOR_FREQ: f32 = 0.55;
/// Décalage vertical de l'ancre en phase 2 (pixels, négatif = plus bas).
/// Assez bas pour que les hearts (Y normalisé ~0.31) soient bien visibles.
const P2_ANCHOR_OFFSET_Y: f32 = -600.0;
/// Durée de la transition vers la nouvelle ancre (secondes).
const P2_TRANSITION_DURATION: f32 = 2.0;
/// Ratio hauteur/largeur du sprite mothership.png.
const MOTHERSHIP_SPRITE_RATIO: f32 = 697.0 / 2048.0;
/// Fraction de la largeur de l'écran occupée par le mothership.
pub(crate) const MOTHERSHIP_SCREEN_FRACTION: f32 = 1.5;

/// Intervalle entre deux frames de l'animation d'apparition (secondes).
pub(crate) const GATLING_ANIM_INTERVAL: f32 = 0.2;

// ─── Silhouette du bord inférieur (convention Top) ─────────────────
pub const MOTHERSHIP_BOTTOM_PROFILE: &[(f32, f32)] = &[
    (-0.50, 0.0),
    (-0.47, -0.1),
    (-0.3, -0.1),
    (-0.15, -0.2),
    (0.0, -0.3),
    (0.15, -0.2),
    (0.3, -0.1),
    (0.47, -0.1),
    (0.50, 0.0),
];

/// Drop table partagée : 10% bombe, 15% bonus score.
pub(crate) static MOTHERSHIP_DROP_TABLE: [(ItemType, f32); 2] =
    [(ItemType::Bomb, 0.10), (ItemType::BonusScore, 0.15)];

// ═══════════════════════════════════════════════════════════════════════
//  Configuration par tourelle
// ═══════════════════════════════════════════════════════════════════════

/// Définition d'un pattern pour une tourelle (version runtime).
#[derive(Clone, Debug)]
pub struct TurretPatternDef {
    pub name: &'static str,
    pub duration: f32,
}

/// Style visuel et gameplay d'une tourelle.
/// Permet de configurer le sprite, le projectile et le laser de visée.
#[derive(Clone, Debug)]
pub struct TurretStyle {
    /// Chemin du sprite (fichier unique, pas de frames d'animation).
    /// Si `None`, utilise les frames animées standard (images/gatling/).
    pub sprite: Option<&'static str>,
    /// Couleur du projectile.
    pub projectile_color: Color,
    /// Vitesse du projectile (pixels/seconde).
    pub projectile_speed: f32,
    /// Rayon de collision du projectile (pixels).
    pub projectile_radius: f32,
    /// Taille visuelle du projectile (largeur, hauteur). Forme pilule si height > width.
    pub projectile_size: Vec2,
    /// Son de tir.
    pub shoot_sound: &'static str,
    /// Volume du son de tir (1.0 = normal).
    pub shoot_sound_volume: f32,
    /// Si true, affiche un laser de visée vers le joueur.
    pub laser: bool,
    /// Couleur du laser de visée.
    pub laser_color: Color,
}

impl Default for TurretStyle {
    fn default() -> Self {
        Self {
            sprite: None,
            projectile_color: Color::rgba(1.0, 0.3, 0.3, 1.0),
            projectile_speed: 450.0,
            projectile_radius: 8.0,
            projectile_size: Vec2::splat(16.0),
            shoot_sound: "audio/sfx/gatling_shoot.ogg",
            shoot_sound_volume: 1.0,
            laser: false,
            laser_color: Color::rgba(0.0, 1.0, 0.0, 0.3),
        }
    }
}

/// Configuration d'une tourelle (patterns + position normalisée sur le sprite).
#[derive(Clone, Debug)]
pub struct TurretConfig {
    pub patterns: Vec<TurretPatternDef>,
    pub pos: Vec2,
    /// Style visuel/gameplay. Si `None`, utilise le style par défaut (gatling rouge).
    pub style: Option<TurretStyle>,
}

impl TurretConfig {
    /// Un seul pattern qui cycle, à une position donnée (style par défaut).
    pub fn single(name: &'static str, duration: f32, pos: Vec2) -> Self {
        Self {
            patterns: vec![TurretPatternDef { name, duration }],
            pos,
            style: None,
        }
    }

    /// Un seul pattern avec un style custom.
    pub fn styled(name: &'static str, duration: f32, pos: Vec2, style: TurretStyle) -> Self {
        Self {
            patterns: vec![TurretPatternDef { name, duration }],
            pos,
            style: Some(style),
        }
    }

    /// Plusieurs patterns qui cyclent dans l'ordre.
    pub fn sequence(patterns: Vec<(&'static str, f32)>, pos: Vec2) -> Self {
        Self {
            patterns: patterns
                .into_iter()
                .map(|(name, duration)| TurretPatternDef { name, duration })
                .collect(),
            pos,
            style: None,
        }
    }
}

/// Configuration complète d'un Mothership.
#[derive(Clone, Debug)]
pub struct MothershipConfig {
    pub edge: SpawnPosition,
    pub turrets: Vec<TurretConfig>,
    pub hearts: Vec<Vec2>,
    pub on_death: Option<Box<MothershipConfig>>,
}

/// File d'attente de configs Mothership à spawner.
#[derive(Resource, Default)]
pub struct MothershipSpawnQueue(pub Vec<MothershipConfig>);

// ═══════════════════════════════════════════════════════════════════════
//  Bord d'apparition
// ═══════════════════════════════════════════════════════════════════════

/// Bord de l'écran depuis lequel le Mothership apparaît.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EntryEdge {
    Top,
    Bottom,
    Left,
    Right,
}

impl EntryEdge {
    pub fn from_spawn_position(sp: SpawnPosition) -> Self {
        match sp {
            SpawnPosition::Top => EntryEdge::Top,
            SpawnPosition::Bottom => EntryEdge::Bottom,
            SpawnPosition::Left => EntryEdge::Left,
            SpawnPosition::Right => EntryEdge::Right,
            SpawnPosition::At(_, _) => EntryEdge::Top,
        }
    }

    pub fn enter_direction(self) -> Vec2 {
        match self {
            EntryEdge::Top => Vec2::new(0.0, -1.0),
            EntryEdge::Bottom => Vec2::new(0.0, 1.0),
            EntryEdge::Left => Vec2::new(1.0, 0.0),
            EntryEdge::Right => Vec2::new(-1.0, 0.0),
        }
    }

    pub fn exit_direction(self) -> Vec2 {
        -self.enter_direction()
    }

    pub fn sprite_rotation(self) -> Quat {
        let angle = match self {
            EntryEdge::Top => 0.0,
            EntryEdge::Bottom => std::f32::consts::PI,
            EntryEdge::Left => std::f32::consts::FRAC_PI_2,
            EntryEdge::Right => -std::f32::consts::FRAC_PI_2,
        };
        Quat::from_rotation_z(angle)
    }

    pub fn mothership_size(self, window_w: f32, window_h: f32) -> Vec2 {
        match self {
            EntryEdge::Top | EntryEdge::Bottom => {
                let w = window_w * MOTHERSHIP_SCREEN_FRACTION;
                Vec2::new(w, w * MOTHERSHIP_SPRITE_RATIO)
            }
            EntryEdge::Left | EntryEdge::Right => {
                let h = window_h * MOTHERSHIP_SCREEN_FRACTION;
                Vec2::new(h * MOTHERSHIP_SPRITE_RATIO, h)
            }
        }
    }

    pub fn transform_offset(self, base: Vec2) -> Vec2 {
        match self {
            EntryEdge::Top => base,
            EntryEdge::Bottom => Vec2::new(base.x, -base.y),
            EntryEdge::Left => Vec2::new(-base.y, base.x),
            EntryEdge::Right => Vec2::new(base.y, base.x),
        }
    }

    pub fn spawn_position(self, half_w: f32, half_h: f32, gatling_total_extent: f32) -> Vec2 {
        match self {
            EntryEdge::Top => Vec2::new(0.0, half_h + gatling_total_extent),
            EntryEdge::Bottom => Vec2::new(0.0, -(half_h + gatling_total_extent)),
            EntryEdge::Left => Vec2::new(-(half_w + gatling_total_extent), 0.0),
            EntryEdge::Right => Vec2::new(half_w + gatling_total_extent, 0.0),
        }
    }

    pub fn gatling_anchor(self) -> bevy::sprite::Anchor {
        bevy::sprite::Anchor::Custom(Vec2::new(0.0, 0.5))
    }

    pub fn cannon_base_atan2(self) -> f32 {
        let dir = self.enter_direction();
        dir.y.atan2(dir.x)
    }

    pub fn is_offscreen(self, pos: Vec3, half_w: f32, half_h: f32) -> bool {
        let margin = 600.0;
        match self {
            EntryEdge::Top => pos.y > half_h + margin,
            EntryEdge::Bottom => pos.y < -(half_h + margin),
            EntryEdge::Left => pos.x < -(half_w + margin),
            EntryEdge::Right => pos.x > half_w + margin,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Composants partagés
// ═══════════════════════════════════════════════════════════════════════

/// Marqueur pour identifier les Gatling parmi les Enemy.
#[derive(Component)]
pub struct GatlingMarker;

/// Lien vers le Mothership parent. Contient l'offset relatif.
#[derive(Component)]
pub struct MothershipLink {
    pub mothership: Entity,
    pub offset: Vec2,
}

/// Marqueur pour le Mothership.
#[derive(Component)]
pub struct MothershipMarker;

/// Marqueur pour les MothershipHearts.
#[derive(Component)]
pub struct MothershipHeart;

/// Si présent, un nouveau Mothership sera spawné à la mort de celui-ci.
#[derive(Component)]
pub struct MothershipSpawnOnDeath(pub MothershipConfig);

/// État et données du Mothership.
#[derive(Component)]
pub struct Mothership {
    pub state: MothershipPhase,
    pub vulnerable: bool,
    pub edge: EntryEdge,
    pub size: Vec2,
    pub anim_timer: Timer,
    pub start_pos: Vec2,
    pub anchor_pos: Vec2,
    pub drift_time: f32,
    pub gatlings: Vec<Entity>,
    pub hearts: Vec<Entity>,
    /// Ancre de départ pour la transition Phase1 → Phase2.
    pub transition_from: Vec2,
    /// Ancre cible pour la phase 2.
    pub transition_to: Vec2,
    /// Timer de la transition Phase1 → Phase2.
    pub transition_timer: Timer,
    /// Timer de l'animation de mort.
    pub dying_timer: Timer,
    /// Position au moment de la mort (pour interpolation).
    pub dying_start_pos: Vec2,
}

/// Phases du Mothership.
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum MothershipPhase {
    /// Arrivée à l'écran.
    Entering,
    /// Phase 1 : gatlings vivantes — drift normal.
    Phase1,
    /// Transition vers Phase 2 : descente lente vers la nouvelle ancre.
    TransitionToPhase2,
    /// Phase 2 : gatlings mortes, hearts restants — flotte plus bas.
    Phase2,
    /// Recule hors de l'écran.
    Dying,
}

/// Override des patterns pour une Gatling.
#[derive(Component)]
pub(crate) struct GatlingPatternOverride {
    pub(crate) patterns: Vec<TurretPatternDef>,
}

/// Style runtime stocké sur chaque Gatling.
#[derive(Component, Clone)]
pub(crate) struct GatlingStyleComp {
    pub(crate) projectile_color: Color,
    pub(crate) projectile_speed: f32,
    pub(crate) projectile_radius: f32,
    pub(crate) projectile_size: Vec2,
    pub(crate) shoot_sound: &'static str,
    pub(crate) shoot_sound_volume: f32,
    pub(crate) has_laser: bool,
    pub(crate) laser_color: Color,
    /// true = sprite statique (pas d'animation par frames).
    pub(crate) static_sprite: bool,
}

impl Default for GatlingStyleComp {
    fn default() -> Self {
        Self {
            projectile_color: Color::rgba(1.0, 0.3, 0.3, 1.0),
            projectile_speed: 450.0,
            projectile_radius: 8.0,
            projectile_size: Vec2::splat(16.0),
            shoot_sound: "audio/sfx/gatling_shoot.ogg",
            shoot_sound_volume: 1.0,
            has_laser: false,
            laser_color: Color::rgba(0.0, 1.0, 0.0, 0.3),
            static_sprite: false,
        }
    }
}

/// Marqueur pour le laser de visée (enfant du Gatling).
#[derive(Component)]
pub(crate) struct GatlingLaser;

/// Biais du centre du cône de visée d'une Gatling (en radians, relatif à la direction de base).
/// Négatif = incliné d'un côté, positif = de l'autre. Permet aux tourelles latérales
/// de viser davantage vers le centre du Mothership.
/// Le second champ est un décalage de phase pour désynchroniser le balayage entre tourelles.
#[derive(Component, Clone, Copy)]
pub(crate) struct GatlingAimBias {
    pub(crate) center_rad: f32,
    pub(crate) phase_offset: f32,
}

/// Frames préchargées de la Gatling (dossier images/gatling/).
#[derive(Resource)]
pub(crate) struct GatlingFrames(pub(crate) Vec<Handle<Image>>);

// ═══════════════════════════════════════════════════════════════════════
//  Helpers
// ═══════════════════════════════════════════════════════════════════════

/// Convertit une position normalisée en offset pixel.
pub(crate) fn turret_pos_to_offset(pos: Vec2, ms_size: Vec2) -> Vec2 {
    Vec2::new(pos.x * ms_size.x, pos.y * ms_size.y)
}

/// Étendue totale depuis le centre du Mothership jusqu'au bout des Gatlings.
fn gatling_total_extent(ms_height: f32) -> f32 {
    ms_height / 2.0 + GATLING_SPRITE_SIZE
}

// ═══════════════════════════════════════════════════════════════════════
//  Préchargement
// ═══════════════════════════════════════════════════════════════════════

pub(crate) fn preload_gatling_frames(mut commands: Commands, asset_server: Res<AssetServer>) {
    let frames = load_frames_from_folder(&asset_server, "images/gatling")
        .expect("gatling frames folder missing or empty");
    commands.insert_resource(GatlingFrames(frames));
}

// ═══════════════════════════════════════════════════════════════════════
//  Spawn Mothership
// ═══════════════════════════════════════════════════════════════════════

/// Consomme les configs dans `MothershipSpawnQueue`.
/// Spawne un Mothership + Gatlings + Hearts.
pub(crate) fn spawn_mothership_oneshot(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut spawn_queue: ResMut<MothershipSpawnQueue>,
    frames: Res<GatlingFrames>,
    windows: Query<&Window>,
) {
    if spawn_queue.0.is_empty() {
        return;
    }
    let config = spawn_queue.0.remove(0);
    info!("Spawning mothership with {} turrets", config.turrets.len());

    let edge = EntryEdge::from_spawn_position(config.edge);
    let window = windows.single();
    let half_w = window.width() / 2.0;
    let half_h = window.height() / 2.0;

    let ms_size = edge.mothership_size(window.width(), window.height());
    let extent = gatling_total_extent(ms_size.y);
    let mut pos = edge.spawn_position(half_w, half_h, extent);
    let rotation = edge.sprite_rotation();

    // Décalage latéral au spawn (le mothership arrive légèrement de la gauche)
    match edge {
        EntryEdge::Top | EntryEdge::Bottom => pos.x += MOTHERSHIP_LATERAL_OFFSET,
        EntryEdge::Left | EntryEdge::Right => pos.y += MOTHERSHIP_LATERAL_OFFSET,
    }

    // ─── Spawn du Mothership ─────────────────────────────────────
    let mothership_texture = asset_server.load("images/mothership/mothership_2.png");

    // Le Mothership est composé de 6 sprites en grille 2x3 (colonne × ligne) :
    //   ┌─────────────┬─────────────┬─────────────┐
    //   │ top-left    │ top-center  │ top-right   │   (ligne haute : flip_y)
    //   │ (flip_x+y)  │ (flip_y)    │ (flip_x+y)  │
    //   ├─────────────┼─────────────┼─────────────┤
    //   │ bottom-left │ bottom-ctr  │ bottom-right│   (ligne basse : rotation de base)
    //   │ (flip_x)    │ (main)      │ (flip_x)    │
    //   └─────────────┴─────────────┴─────────────┘
    let mirror_top_center_offset = Vec3::new(0.0, ms_size.y, -0.01);
    let mirror_bottom_left_offset = Vec3::new(-ms_size.x, 0.0, -0.01);
    let mirror_bottom_right_offset = Vec3::new(ms_size.x, 0.0, -0.01);
    let mirror_top_left_offset = Vec3::new(-ms_size.x, ms_size.y, -0.01);
    let mirror_top_right_offset = Vec3::new(ms_size.x, ms_size.y, -0.01);

    let mothership_entity = commands
        .spawn((
            SpriteBundle {
                texture: mothership_texture.clone(),
                sprite: Sprite {
                    custom_size: Some(ms_size),
                    ..default()
                },
                transform: Transform {
                    translation: Vec3::new(pos.x, pos.y, 0.4),
                    rotation,
                    ..default()
                },
                ..default()
            },
            MothershipMarker,
        ))
        .with_children(|parent| {
            // Haut-centre : flip Y
            parent.spawn(SpriteBundle {
                texture: mothership_texture.clone(),
                sprite: Sprite {
                    custom_size: Some(ms_size),
                    flip_y: true,
                    ..default()
                },
                transform: Transform::from_translation(mirror_top_center_offset),
                ..default()
            });
            // Bas-gauche : flip X
            parent.spawn(SpriteBundle {
                texture: mothership_texture.clone(),
                sprite: Sprite {
                    custom_size: Some(ms_size),
                    flip_x: true,
                    ..default()
                },
                transform: Transform::from_translation(mirror_bottom_left_offset),
                ..default()
            });
            // Bas-droite : flip X
            parent.spawn(SpriteBundle {
                texture: mothership_texture.clone(),
                sprite: Sprite {
                    custom_size: Some(ms_size),
                    flip_x: true,
                    ..default()
                },
                transform: Transform::from_translation(mirror_bottom_right_offset),
                ..default()
            });
            // Haut-gauche : flip X + flip Y (même rotation que haut-centre)
            parent.spawn(SpriteBundle {
                texture: mothership_texture.clone(),
                sprite: Sprite {
                    custom_size: Some(ms_size),
                    flip_x: true,
                    flip_y: true,
                    ..default()
                },
                transform: Transform::from_translation(mirror_top_left_offset),
                ..default()
            });
            // Haut-droite : flip X + flip Y
            parent.spawn(SpriteBundle {
                texture: mothership_texture,
                sprite: Sprite {
                    custom_size: Some(ms_size),
                    flip_x: true,
                    flip_y: true,
                    ..default()
                },
                transform: Transform::from_translation(mirror_top_right_offset),
                ..default()
            });
        })
        .id();

    // ─── Spawn des Gatlings ──────────────────────────────────────
    let ms_size_top = match edge {
        EntryEdge::Top | EntryEdge::Bottom => ms_size,
        EntryEdge::Left | EntryEdge::Right => Vec2::new(ms_size.y, ms_size.x),
    };
    let mut gatling_entities = Vec::with_capacity(config.turrets.len().max(1));

    for (_i, turret_cfg) in config.turrets.iter().enumerate() {
        let base_offset = turret_pos_to_offset(turret_cfg.pos, ms_size_top);
        let offset = edge.transform_offset(base_offset);
        let gatling_pos = Vec2::new(pos.x + offset.x, pos.y + offset.y);
        let phase = &GATLING.phases[0];

        // Biais de visée : la tourelle regarde davantage vers le centre du Mothership.
        // `offset` est le vecteur centre→tourelle en monde ; direction vers le centre = -offset.
        // On calcule l'angle relatif par rapport à la direction de base, puis on le borne à ±20°
        // pour éviter une visée trop oblique.
        let aim_bias = {
            let to_center = -Vec2::new(offset.x, offset.y);
            if to_center.length_squared() > 1.0 {
                let base_dir = edge.enter_direction();
                let to_center_n = to_center.normalize();
                let base_atan2 = base_dir.y.atan2(base_dir.x);
                let to_center_atan2 = to_center_n.y.atan2(to_center_n.x);
                let mut rel = to_center_atan2 - base_atan2;
                while rel > std::f32::consts::PI {
                    rel -= std::f32::consts::TAU;
                }
                while rel < -std::f32::consts::PI {
                    rel += std::f32::consts::TAU;
                }
                rel.clamp(-20f32.to_radians(), 20f32.to_radians())
            } else {
                0.0
            }
        };
        // Phase : inverse selon le côté du mothership pour désynchroniser les balayages.
        let phase_offset = if turret_cfg.pos.x >= 0.0 {
            std::f32::consts::PI
        } else {
            0.0
        };
        let aim_bias_comp = crate::enemy::mothership::GatlingAimBias {
            center_rad: aim_bias,
            phase_offset,
        };

        // Sprite : statique (TurretStyle.sprite) ou animé (frames standard)
        let style_comp = if let Some(ref style) = turret_cfg.style {
            GatlingStyleComp {
                projectile_color: style.projectile_color,
                projectile_speed: style.projectile_speed,
                projectile_radius: style.projectile_radius,
                projectile_size: style.projectile_size,
                shoot_sound: style.shoot_sound,
                shoot_sound_volume: style.shoot_sound_volume,
                has_laser: style.laser,
                laser_color: style.laser_color,
                static_sprite: style.sprite.is_some(),
            }
        } else {
            GatlingStyleComp::default()
        };

        let texture_handle: Handle<Image> = if let Some(ref style) = turret_cfg.style {
            if let Some(path) = style.sprite {
                asset_server.load(path)
            } else {
                frames.0.first().cloned().unwrap_or_default()
            }
        } else {
            frames.0.first().cloned().unwrap_or_default()
        };

        let mut entity_cmds = commands.spawn((
            SpriteBundle {
                texture: texture_handle,
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(GATLING_SPRITE_SIZE)),
                    anchor: edge.gatling_anchor(),
                    flip_y: style_comp.static_sprite,
                    ..default()
                },
                transform: Transform {
                    translation: Vec3::new(gatling_pos.x, gatling_pos.y, 0.5),
                    rotation,
                    ..default()
                },
                ..default()
            },
            Enemy {
                state: EnemyState::Entering,
                radius: GATLING.radius,
                sprite_size: GATLING.sprite_size,
                anim_timer: Timer::from_seconds(MOTHERSHIP_ENTERING_DURATION, TimerMode::Once),
                phases: GATLING.phases,
                death_duration: GATLING.death_duration,
                death_shake_max: GATLING.death_shake_max,
                hit_sound: GATLING.hit_sound,
                death_explosion_sound: GATLING.death_explosion_sound,
                hit_flash_color: None,
            },
            crate::physic::health::Health::new(phase.health),
            GatlingMarker,
            crate::enemy::gatling::GatlingBaseEdge(edge),
            MothershipLink {
                mothership: mothership_entity,
                offset,
            },
            crate::enemy::gatling::GatlingEnteringAnim {
                timer: Timer::from_seconds(GATLING_ANIM_INTERVAL, TimerMode::Repeating),
                current_frame: 0,
            },
            PatternIndex(0),
            PatternTimer(Timer::from_seconds(0.0, TimerMode::Once)),
            DropTable {
                drops: &MOTHERSHIP_DROP_TABLE,
            },
            style_comp.clone(),
            aim_bias_comp,
        ));

        entity_cmds.insert(GatlingPatternOverride {
            patterns: turret_cfg.patterns.clone(),
        });

        // Laser de visée
        let gatling_id = entity_cmds.id();
        if style_comp.has_laser {
            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: style_comp.laser_color,
                        custom_size: Some(Vec2::new(2.0, 1.0)), // Sera redimensionné dynamiquement
                        anchor: bevy::sprite::Anchor::Custom(Vec2::new(0.0, -0.5)),
                        ..default()
                    },
                    transform: Transform::from_xyz(gatling_pos.x, gatling_pos.y, 0.49),
                    ..default()
                },
                GatlingLaser,
                MothershipLink {
                    mothership: gatling_id,
                    offset: Vec2::ZERO,
                },
            ));
        }

        gatling_entities.push(gatling_id);
    }

    // ─── Spawn des Hearts ────────────────────────────────────────
    let heart_phase = &MOTHERSHIP_HEART.phases[0];
    let mut heart_entities = Vec::with_capacity(config.hearts.len());
    for heart_pos in &config.hearts {
        let base_offset = turret_pos_to_offset(*heart_pos, ms_size_top);
        let offset = edge.transform_offset(base_offset);
        let heart_world = Vec2::new(pos.x + offset.x, pos.y + offset.y);

        let heart_entity = commands
            .spawn((
                SpriteBundle {
                    texture: asset_server.load("images/mothership/mothership_heart.png"),
                    sprite: Sprite {
                        custom_size: Some(Vec2::splat(MOTHERSHIP_HEART.sprite_size)),
                        ..default()
                    },
                    transform: Transform {
                        translation: Vec3::new(heart_world.x, heart_world.y, 0.45),
                        rotation,
                        ..default()
                    },
                    ..default()
                },
                Enemy {
                    state: EnemyState::Active(0),
                    radius: MOTHERSHIP_HEART.radius,
                    sprite_size: MOTHERSHIP_HEART.sprite_size,
                    anim_timer: Timer::from_seconds(
                        MOTHERSHIP_HEART.death_duration,
                        TimerMode::Once,
                    ),
                    phases: MOTHERSHIP_HEART.phases,
                    death_duration: MOTHERSHIP_HEART.death_duration,
                    death_shake_max: MOTHERSHIP_HEART.death_shake_max,
                    hit_sound: MOTHERSHIP_HEART.hit_sound,
                    death_explosion_sound: MOTHERSHIP_HEART.death_explosion_sound,
                    hit_flash_color: Some(Color::YELLOW),
                },
                crate::physic::health::Health::new(heart_phase.health),
                MothershipHeart,
                MothershipLink {
                    mothership: mothership_entity,
                    offset,
                },
                PatternIndex(0),
                PatternTimer(Timer::from_seconds(999.0, TimerMode::Once)),
                DropTable {
                    drops: &MOTHERSHIP_DROP_TABLE,
                },
            ))
            .id();
        heart_entities.push(heart_entity);
    }

    // ─── Insérer le composant Mothership ─────────────────────────
    // L'ancre (position de repos) = position de spawn + distance d'entrée
    // L'overshoot ira un peu plus loin, puis la stabilisation ramène à l'ancre.
    let dir = edge.enter_direction();
    let anchor = pos + dir * MOTHERSHIP_ENTERING_DISTANCE;
    commands.entity(mothership_entity).insert(Mothership {
        state: MothershipPhase::Entering,
        vulnerable: false,
        edge,
        size: ms_size,
        anim_timer: Timer::from_seconds(MOTHERSHIP_ENTERING_DURATION, TimerMode::Once),
        start_pos: pos,
        anchor_pos: anchor,
        drift_time: 0.0,
        gatlings: gatling_entities,
        hearts: heart_entities,
        transition_from: Vec2::ZERO,
        transition_to: Vec2::ZERO,
        transition_timer: Timer::from_seconds(P2_TRANSITION_DURATION, TimerMode::Once),
        dying_timer: Timer::from_seconds(MOTHERSHIP_DYING_DURATION, TimerMode::Once),
        dying_start_pos: Vec2::ZERO,
    });

    if let Some(next) = config.on_death {
        commands
            .entity(mothership_entity)
            .insert(MothershipSpawnOnDeath(*next));
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Mothership Entering
// ═══════════════════════════════════════════════════════════════════════

pub(crate) fn mothership_entering(
    time: Res<Time>,
    mut mothership_q: Query<(&mut Mothership, &mut Transform), With<MothershipMarker>>,
    mut enemy_q: Query<(&mut Enemy, &mut crate::physic::health::Health), With<GatlingMarker>>,
) {
    for (mut mothership, mut transform) in mothership_q.iter_mut() {
        if mothership.state != MothershipPhase::Entering {
            continue;
        }

        mothership.anim_timer.tick(time.delta());
        let dt = time.delta_seconds();
        let progress = mothership.anim_timer.fraction();

        // Ease-out avec forte inertie : arrive très vite, décélère lourdement.
        let eased = 1.0 - (1.0 - progress).powi(MOTHERSHIP_INERTIA_POWER);

        let dir = mothership.edge.enter_direction();

        // Déplacement vertical (axe d'entrée)
        let vert = dir * MOTHERSHIP_ENTER_VERTICAL * eased;
        // Déplacement latéral (rattrape le décalage vers le centre)
        let lat = match mothership.edge {
            EntryEdge::Top | EntryEdge::Bottom => Vec2::new(MOTHERSHIP_ENTER_LATERAL * eased, 0.0),
            EntryEdge::Left | EntryEdge::Right => Vec2::new(0.0, MOTHERSHIP_ENTER_LATERAL * eased),
        };

        // Accumuler drift_time pendant l'entering pour que le drift
        // ait déjà de la vélocité quand Phase1 commence.
        mothership.drift_time += dt;

        // Blender le drift progressivement dans la 2e moitié de l'entering.
        // drift_blend : 0 → 1 sur la plage [0.5..1.0] du progress.
        let drift_blend = ((progress - 0.5) * 2.0).clamp(0.0, 1.0);
        let t = mothership.drift_time;
        let drift_main = (t * P1_DRIFT_MAIN_FREQ).sin() * P1_DRIFT_MAIN_AMP * drift_blend;
        let drift_minor = (t * P1_DRIFT_MINOR_FREQ).sin() * P1_DRIFT_MINOR_AMP * drift_blend;
        let drift_offset = match mothership.edge {
            EntryEdge::Top | EntryEdge::Bottom => Vec2::new(drift_main, drift_minor),
            EntryEdge::Left | EntryEdge::Right => Vec2::new(drift_minor, drift_main),
        };

        let displacement = vert + lat;
        let base_pos = mothership.start_pos + displacement;
        transform.translation.x = base_pos.x + drift_offset.x;
        transform.translation.y = base_pos.y + drift_offset.y;

        if mothership.anim_timer.finished() {
            // L'ancre = position de base (sans le drift), pour que le drift
            // system ajoute exactement le même offset → pas de saut.
            mothership.anchor_pos = base_pos;
            mothership.state = MothershipPhase::Phase1;

            // Activer toutes les Gatlings
            for &gatling_entity in &mothership.gatlings {
                if let Ok((mut enemy, mut health)) = enemy_q.get_mut(gatling_entity) {
                    let phase_def = &enemy.phases[0];
                    enemy.state = EnemyState::Active(0);
                    health.reset(phase_def.health);
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Flottement (drift)
// ═══════════════════════════════════════════════════════════════════════

pub(crate) fn mothership_drift(
    time: Res<Time>,
    mut query: Query<(&mut Mothership, &mut Transform), With<MothershipMarker>>,
) {
    for (mut mothership, mut transform) in query.iter_mut() {
        // Paramètres de drift + blend ratio selon la phase
        let blend = match mothership.state {
            MothershipPhase::Phase1 => 0.0, // 100% P1
            MothershipPhase::TransitionToPhase2 => {
                // Descend l'ancre progressivement
                mothership.transition_timer.tick(time.delta());
                let progress = mothership.transition_timer.fraction();
                let eased = 1.0 - (1.0 - progress).powi(3);
                mothership.anchor_pos = mothership
                    .transition_from
                    .lerp(mothership.transition_to, eased);

                if mothership.transition_timer.finished() {
                    mothership.anchor_pos = mothership.transition_to;
                    mothership.state = MothershipPhase::Phase2;
                }

                eased // 0→1 blend P1→P2
            }
            MothershipPhase::Phase2 => 1.0, // 100% P2
            _ => continue,
        };

        // Interpoler les paramètres de drift pour une transition fluide
        let main_amp = P1_DRIFT_MAIN_AMP + (P2_DRIFT_MAIN_AMP - P1_DRIFT_MAIN_AMP) * blend;
        let minor_amp = P1_DRIFT_MINOR_AMP + (P2_DRIFT_MINOR_AMP - P1_DRIFT_MINOR_AMP) * blend;
        let main_freq = P1_DRIFT_MAIN_FREQ + (P2_DRIFT_MAIN_FREQ - P1_DRIFT_MAIN_FREQ) * blend;
        let minor_freq = P1_DRIFT_MINOR_FREQ + (P2_DRIFT_MINOR_FREQ - P1_DRIFT_MINOR_FREQ) * blend;

        mothership.drift_time += time.delta_seconds();
        let t = mothership.drift_time;

        let main_offset = (t * main_freq).sin() * main_amp;
        let minor_offset = (t * minor_freq).sin() * minor_amp;

        match mothership.edge {
            EntryEdge::Top | EntryEdge::Bottom => {
                transform.translation.x = mothership.anchor_pos.x + main_offset;
                transform.translation.y = mothership.anchor_pos.y + minor_offset;
            }
            EntryEdge::Left | EntryEdge::Right => {
                transform.translation.x = mothership.anchor_pos.x + minor_offset;
                transform.translation.y = mothership.anchor_pos.y + main_offset;
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Synchronisation positions (Gatlings + Hearts)
// ═══════════════════════════════════════════════════════════════════════

pub(crate) fn mothership_sync_positions(
    mut commands: Commands,
    mothership_q: Query<
        &Transform,
        (
            With<MothershipMarker>,
            Without<GatlingMarker>,
            Without<MothershipHeart>,
        ),
    >,
    mut gatling_q: Query<
        (&MothershipLink, &Enemy, &mut Transform),
        (
            With<GatlingMarker>,
            Without<MothershipMarker>,
            Without<MothershipHeart>,
        ),
    >,
    mut heart_q: Query<
        (Entity, &MothershipLink, &Enemy, &mut Transform),
        (
            With<MothershipHeart>,
            Without<MothershipMarker>,
            Without<GatlingMarker>,
        ),
    >,
) {
    for (link, enemy, mut transform) in gatling_q.iter_mut() {
        if matches!(enemy.state, EnemyState::Dying | EnemyState::Dead) {
            continue;
        }
        if let Ok(ms_transform) = mothership_q.get(link.mothership) {
            transform.translation.x = ms_transform.translation.x + link.offset.x;
            transform.translation.y = ms_transform.translation.y + link.offset.y;
        }
    }

    for (entity, link, enemy, mut transform) in heart_q.iter_mut() {
        if matches!(enemy.state, EnemyState::Dying | EnemyState::Dead) {
            continue;
        }
        if let Ok(ms_transform) = mothership_q.get(link.mothership) {
            transform.translation.x = ms_transform.translation.x + link.offset.x;
            transform.translation.y = ms_transform.translation.y + link.offset.y;
        } else {
            if let Some(e) = commands.get_entity(entity) {
                e.despawn_recursive();
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Détection de mort
// ═══════════════════════════════════════════════════════════════════════

pub(crate) fn mothership_death_detection(
    mut mothership_q: Query<(&mut Mothership, &Transform), With<MothershipMarker>>,
    enemy_q: Query<&Enemy>,
) {
    for (mut mothership, transform) in mothership_q.iter_mut() {
        let is_dead = |e: &Entity| match enemy_q.get(*e) {
            Ok(enemy) => matches!(enemy.state, EnemyState::Dying | EnemyState::Dead),
            Err(_) => true,
        };

        match mothership.state {
            MothershipPhase::Phase1 => {
                // Toutes les gatlings mortes → transition vers Phase 2
                if mothership.gatlings.iter().all(is_dead) {
                    let from = mothership.anchor_pos;
                    let dir = mothership.edge.enter_direction();
                    let offset = match mothership.edge {
                        EntryEdge::Top | EntryEdge::Bottom => {
                            Vec2::new(0.0, dir.y * P2_ANCHOR_OFFSET_Y.abs())
                        }
                        EntryEdge::Left | EntryEdge::Right => {
                            Vec2::new(dir.x * P2_ANCHOR_OFFSET_Y.abs(), 0.0)
                        }
                    };
                    mothership.transition_from = from;
                    mothership.transition_to = from + offset;
                    mothership.transition_timer =
                        Timer::from_seconds(P2_TRANSITION_DURATION, TimerMode::Once);
                    mothership.state = MothershipPhase::TransitionToPhase2;
                }
            }
            MothershipPhase::TransitionToPhase2 | MothershipPhase::Phase2 => {
                // Tous les hearts morts → Dying
                if mothership.hearts.iter().all(is_dead) {
                    mothership.dying_start_pos = transform.translation.truncate();
                    mothership.dying_timer =
                        Timer::from_seconds(MOTHERSHIP_DYING_DURATION, TimerMode::Once);
                    mothership.state = MothershipPhase::Dying;
                }
            }
            _ => {}
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Dying : recule vers le bord d'origine
// ═══════════════════════════════════════════════════════════════════════

pub(crate) fn mothership_dying(
    mut commands: Commands,
    time: Res<Time>,
    mut mothership_q: Query<
        (
            Entity,
            &mut Mothership,
            &mut Transform,
            &mut Sprite,
            Option<&MothershipSpawnOnDeath>,
        ),
        With<MothershipMarker>,
    >,
    mut level_events: EventWriter<LevelActionEvent>,
    mut spawn_queue: ResMut<MothershipSpawnQueue>,
) {
    let mut to_despawn: Vec<(Entity, Option<MothershipConfig>)> = Vec::new();

    for (entity, mut mothership, mut transform, mut sprite, spawn_on_death) in
        mothership_q.iter_mut()
    {
        if mothership.state != MothershipPhase::Dying {
            continue;
        }

        mothership.dying_timer.tick(time.delta());
        let progress = mothership.dying_timer.fraction();
        // Ease-in : démarre lentement, accélère à la fin (sensation de masse)
        let eased = progress * progress;

        let exit_dir = mothership.edge.exit_direction();
        let total_distance = MOTHERSHIP_ENTERING_DISTANCE + 800.0;
        let target = mothership.dying_start_pos + exit_dir * total_distance;
        let pos = mothership.dying_start_pos.lerp(target, eased);
        transform.translation.x = pos.x;
        transform.translation.y = pos.y;

        // Fade out progressif
        sprite.color.set_a(0.8 * (1.0 - progress));

        if mothership.dying_timer.finished() {
            let next_config = spawn_on_death.map(|s| s.0.clone());
            to_despawn.push((entity, next_config));
        }
    }

    for (entity, next_config) in to_despawn {
        if let Some(config) = next_config {
            spawn_queue.0.push(config);
        } else {
            level_events.send(LevelActionEvent(vec![Action::MarkLevelComplete]));
        }
        if let Some(e) = commands.get_entity(entity) {
            e.despawn_recursive();
        }
    }
}
