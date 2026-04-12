//! Gatling & Mothership — ennemis utilisant le framework `enemy.rs`.
//!
//! Le Mothership est un vaisseau mère avec 3 Gatlings attachées.
//! Il peut apparaître depuis n'importe quel bord de l'écran (`EntryEdge`).
//!
//! Machine à état Mothership : Entering (3s) → Active → Dying (recule vers le bord)
//! Machine à état Gatling : Entering (animation sprite) → Active(0) → Dying → Dead
//!
//! Les Gatlings sont des entités indépendantes avec leur propre hitbox,
//! dont la position est synchronisée au Mothership via `MothershipLink`.
//!
//! Le Mothership est immortel par défaut (`vulnerable: false`).
//! Quand toutes les Gatlings meurent, le Mothership recule hors de l'écran
//! puis envoie `MarkLevelComplete`.

use crate::difficulty::SpawnPosition;
use crate::enemies::GATLING;
use crate::enemy::{Enemy, EnemyProjectile, EnemyState, PatternIndex, PatternTimer};
use crate::explosion::load_frames_from_folder;
use crate::item::{DropTable, ItemType};
use crate::level::{Action, LevelActionEvent};
use crate::pause::not_paused;
use crate::player::Player;
use crate::state::GameState;
use bevy::prelude::*;

pub struct GatlingPlugin;

impl Plugin for GatlingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MothershipSpawnQueue>()
            .add_systems(Startup, preload_gatling_frames)
            .add_systems(
                Update,
                (
                    spawn_mothership_oneshot,
                    spawn_gatlings_oneshot,
                    mothership_entering,
                    mothership_drift,
                    mothership_sync_positions,
                    mothership_death_detection,
                    mothership_dying,
                    gatling_standalone_entering,
                    gatling_entering_animate,
                    gatling_pattern_executor,
                    gatling_shoot_update,
                    gatling_full_auto_update,
                )
                    .run_if(in_state(GameState::Playing))
                    .run_if(not_paused),
            );
    }
}

// ─── Constantes ─────────────────────────────────────────────────────

/// Intervalle entre deux frames de l'animation d'apparition (secondes).
const GATLING_ANIM_INTERVAL: f32 = 0.2;
/// Durée de la phase Entering (secondes).
const GATLING_ENTERING_DURATION: f32 = 3.0;
/// Distance parcourue pendant l'Entering (pixels).
/// 2.5 tiles = 320px pour que le mothership soit bien visible.
const GATLING_ENTERING_DISTANCE: f32 = 320.0;
/// Taille du sprite Gatling (pixels).
const GATLING_SPRITE_SIZE: f32 = 128.0;

/// Espacement entre les Gatlings le long de l'axe perpendiculaire (pixels).
const GATLING_SPACING: f32 = 200.0;
/// Vitesse de recul du Mothership pendant la mort (pixels/seconde).
const MOTHERSHIP_DYING_SPEED: f32 = 100.0;

// ─── Flottement du Mothership (à ajuster) ───────────────────────
// Axe principal = perpendiculaire au bord d'entrée (gauche↔droite pour Top/Bottom).
// Axe secondaire = parallèle au bord d'entrée (haut↔bas pour Top/Bottom).
/// Amplitude du flottement principal (pixels).
const MOTHERSHIP_DRIFT_MAIN_AMP: f32 = 60.0;
/// Amplitude du flottement secondaire (pixels).
const MOTHERSHIP_DRIFT_MINOR_AMP: f32 = 15.0;
/// Fréquence du flottement principal (rad/s). Plus haut = plus rapide.
const MOTHERSHIP_DRIFT_MAIN_FREQ: f32 = 0.5;
/// Fréquence du flottement secondaire (rad/s).
const MOTHERSHIP_DRIFT_MINOR_FREQ: f32 = 0.3;
/// Taille de base du placeholder Mothership (7×2 tiles de 128px).
/// Pour Top/Bottom : largeur × hauteur. Pour Left/Right : inversé automatiquement.
const MOTHERSHIP_BASE_SIZE: Vec2 = Vec2::new(896.0, 256.0);

// ─── Tir de la Gatling (à ajuster) ──────────────────────────────
/// Angle de rotation max vers le joueur (degrés).
const GATLING_AIM_MAX_ANGLE: f32 = 40.0;
/// Vitesse de rotation vers le joueur (degrés/seconde).
const GATLING_AIM_SPEED: f32 = 60.0;
/// Vitesse du projectile (pixels/seconde).
const GATLING_PROJECTILE_SPEED: f32 = 400.0;
/// Rayon de collision du projectile (pixels).
const GATLING_PROJECTILE_RADIUS: f32 = 8.0;
/// Intervalle entre deux frames de l'animation de tir (secondes).
const GATLING_SHOOT_ANIM_INTERVAL: f32 = 0.1;

// ─── Full Auto (à ajuster) ──────────────────────────────────────
/// Vitesse de balayage initiale (degrés/seconde).
const FULL_AUTO_SWEEP_SPEED_START: f32 = 30.0;
/// Vitesse de balayage maximale (degrés/seconde).
const FULL_AUTO_SWEEP_SPEED_MAX: f32 = 180.0;
/// Intervalle de tir initial (secondes entre chaque tir).
const FULL_AUTO_FIRE_INTERVAL_START: f32 = 0.8;
/// Intervalle de tir minimal (cadence max, doit rester > temps d'animation).
const FULL_AUTO_FIRE_INTERVAL_MIN: f32 = 0.15;
/// Courbe d'accélération (>1 = montée lente, <1 = montée rapide, 1 = linéaire).
const FULL_AUTO_RAMP_FACTOR: f32 = 1.5;
/// Intervalle entre deux frames de l'animation de tir en full auto (secondes).
/// Plus rapide que le shoot normal pour accompagner la cadence.
const FULL_AUTO_SHOOT_ANIM_INTERVAL: f32 = 0.04;

/// Drop table : 10% bombe, 15% bonus score.
static GATLING_DROP_TABLE: [(ItemType, f32); 2] =
    [(ItemType::Bomb, 0.10), (ItemType::BonusScore, 0.15)];

// ─── Configuration par tourelle ─────────────────────────────────────

/// Définition d'un pattern pour une tourelle (version runtime, non statique).
#[derive(Clone, Debug)]
pub struct TurretPatternDef {
    pub name: &'static str,
    pub duration: f32,
}

/// Configuration d'une tourelle (liste de patterns qui cyclent).
#[derive(Clone, Debug)]
pub struct TurretConfig {
    pub patterns: Vec<TurretPatternDef>,
}

/// Configuration complète d'un Mothership.
/// `turrets` : un `TurretConfig` par tourelle (gauche, centre, droite).
/// Si moins de 3, les tourelles manquantes reprennent la dernière config.
#[derive(Clone, Debug)]
pub struct MothershipConfig {
    pub edge: SpawnPosition,
    pub turrets: Vec<TurretConfig>,
}

/// Helpers pour construire un `TurretConfig` rapidement.
impl TurretConfig {
    /// Un seul pattern qui cycle.
    pub fn single(name: &'static str, duration: f32) -> Self {
        Self {
            patterns: vec![TurretPatternDef { name, duration }],
        }
    }

    /// Plusieurs patterns qui cyclent dans l'ordre.
    pub fn sequence(patterns: Vec<(&'static str, f32)>) -> Self {
        Self {
            patterns: patterns
                .into_iter()
                .map(|(name, duration)| TurretPatternDef { name, duration })
                .collect(),
        }
    }
}

/// File d'attente de configs Mothership à spawner.
#[derive(Resource, Default)]
pub struct MothershipSpawnQueue(pub Vec<MothershipConfig>);

/// Override des patterns pour une Gatling (remplace enemy.phases).
#[derive(Component)]
struct GatlingPatternOverride {
    patterns: Vec<TurretPatternDef>,
}

// ─── Bord d'apparition ─────────────────────────────────────────────

/// Bord de l'écran depuis lequel le Mothership apparaît.
/// Détermine la direction d'entrée, la disposition des Gatlings,
/// et la direction de fuite à la mort.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EntryEdge {
    Top,
    Bottom,
    Left,
    Right,
}

impl EntryEdge {
    /// Déduit le bord d'entrée depuis un `SpawnPosition`.
    /// `At(_, _)` utilise `Top` par défaut.
    pub fn from_spawn_position(sp: SpawnPosition) -> Self {
        match sp {
            SpawnPosition::Top => EntryEdge::Top,
            SpawnPosition::Bottom => EntryEdge::Bottom,
            SpawnPosition::Left => EntryEdge::Left,
            SpawnPosition::Right => EntryEdge::Right,
            SpawnPosition::At(_, _) => EntryEdge::Top,
        }
    }

    /// Direction de déplacement pendant l'Entering (vers l'intérieur de l'écran).
    pub fn enter_direction(self) -> Vec2 {
        match self {
            EntryEdge::Top => Vec2::new(0.0, -1.0),
            EntryEdge::Bottom => Vec2::new(0.0, 1.0),
            EntryEdge::Left => Vec2::new(1.0, 0.0),
            EntryEdge::Right => Vec2::new(-1.0, 0.0),
        }
    }

    /// Direction de fuite pendant le Dying (vers l'extérieur de l'écran).
    pub fn exit_direction(self) -> Vec2 {
        -self.enter_direction()
    }

    /// Rotation à appliquer aux sprites (Mothership et Gatlings).
    /// Top = 0°, Bottom = 180°, Left = -90°, Right = +90°.
    pub fn sprite_rotation(self) -> Quat {
        let angle = match self {
            EntryEdge::Top => 0.0,
            EntryEdge::Bottom => std::f32::consts::PI,
            EntryEdge::Left => std::f32::consts::FRAC_PI_2,
            EntryEdge::Right => -std::f32::consts::FRAC_PI_2,
        };
        Quat::from_rotation_z(angle)
    }

    /// Taille du sprite Mothership, inversée pour Left/Right.
    pub fn sprite_size(self) -> Vec2 {
        match self {
            EntryEdge::Top | EntryEdge::Bottom => MOTHERSHIP_BASE_SIZE,
            EntryEdge::Left | EntryEdge::Right => {
                Vec2::new(MOTHERSHIP_BASE_SIZE.y, MOTHERSHIP_BASE_SIZE.x)
            }
        }
    }

    /// Transforme un offset Gatling de base (conçu pour `Top`) vers ce bord.
    ///
    /// Convention base (Top) : `x` = spread horizontal, `y` = profondeur (négatif = vers l'écran).
    /// - Bottom : flip Y
    /// - Left   : rotation 90° CCW → profondeur vers +X, spread vers Y
    /// - Right  : rotation 90° CW  → profondeur vers -X, spread vers Y
    pub fn transform_offset(self, base: Vec2) -> Vec2 {
        match self {
            EntryEdge::Top => base,
            EntryEdge::Bottom => Vec2::new(base.x, -base.y),
            EntryEdge::Left => Vec2::new(-base.y, base.x),
            EntryEdge::Right => Vec2::new(base.y, base.x),
        }
    }

    /// Position de spawn (centre du Mothership, entièrement hors écran).
    /// Le point le plus avancé (bout des Gatlings) est juste au bord.
    pub fn spawn_position(self, half_w: f32, half_h: f32, gatling_total_extent: f32) -> Vec2 {
        match self {
            EntryEdge::Top => Vec2::new(0.0, half_h + gatling_total_extent),
            EntryEdge::Bottom => Vec2::new(0.0, -(half_h + gatling_total_extent)),
            EntryEdge::Left => Vec2::new(-(half_w + gatling_total_extent), 0.0),
            EntryEdge::Right => Vec2::new(half_w + gatling_total_extent, 0.0),
        }
    }

    /// Ancre du sprite Gatling : le point d'attache au Mothership.
    /// C'est le pivot de rotation pour le pattern "shoot".
    /// En convention Bevy : (0,0) = centre, (0, 0.5) = haut, (0, -0.5) = bas.
    pub fn gatling_anchor(self) -> bevy::sprite::Anchor {
        // La rotation du sprite est appliquée AVANT l'anchor. Puisqu'on fait pivoter
        // le sprite pour qu'il pointe dans la direction d'entrée, l'anchor doit
        // toujours être "le haut" (côté mothership = opposé au bout du canon).
        // Top: sprite non tourné → haut = (0, 0.5)
        // Bottom: sprite tourné 180° → haut original est en bas → anchor = (0, 0.5) aussi
        //   car Bevy applique anchor AVANT rotation dans le rendu
        // En fait, l'anchor est en coordonnées locales du sprite (avant rotation),
        // donc c'est toujours le haut du sprite original.
        bevy::sprite::Anchor::Custom(Vec2::new(0.0, 0.5))
    }

    /// Angle absolu de la direction d'entrée (atan2), utilisé comme référence
    /// pour le calcul de visée. C'est l'angle du canon au repos.
    pub fn cannon_base_atan2(self) -> f32 {
        let dir = self.enter_direction();
        dir.y.atan2(dir.x)
    }

    /// Vérifie si le Mothership est entièrement sorti de l'écran (pendant le Dying).
    pub fn is_offscreen(self, pos: Vec3, half_w: f32, half_h: f32) -> bool {
        let size = self.sprite_size();
        match self {
            EntryEdge::Top => pos.y > half_h + size.y,
            EntryEdge::Bottom => pos.y < -(half_h + size.y),
            EntryEdge::Left => pos.x < -(half_w + size.x),
            EntryEdge::Right => pos.x > half_w + size.x,
        }
    }
}

// ─── Composants ─────────────────────────────────────────────────────

/// Marqueur pour identifier les Gatling parmi les Enemy.
#[derive(Component)]
pub struct GatlingMarker;

/// Lien vers le Mothership parent. Contient l'offset relatif.
#[derive(Component)]
pub struct MothershipLink {
    pub mothership: Entity,
    pub offset: Vec2,
}

/// Animation de la Gatling pendant l'Entering.
#[derive(Component)]
struct GatlingEnteringAnim {
    timer: Timer,
    current_frame: usize,
}

/// Position Y de départ pour un Gatling standalone (sans Mothership).
#[derive(Component)]
struct GatlingStartY(f32);

/// Composant actif pendant le pattern "aim_and_shoot".
/// Suit le joueur en continu, puis tire vers la fin du pattern.
#[derive(Component)]
struct GatlingShoot {
    /// Angle relatif cible (radians, par rapport au repos, clamped à ±max).
    target_angle: f32,
    /// Angle relatif courant (radians, interpolé vers target_angle).
    current_angle: f32,
    /// Temps écoulé depuis le début du pattern.
    elapsed: f32,
    /// Durée totale du pattern (secondes).
    duration: f32,
    /// Animation de tir.
    anim_timer: Timer,
    /// Frame courante de l'animation de tir.
    current_frame: usize,
    /// Le projectile a déjà été tiré.
    fired: bool,
    /// L'animation de tir a commencé.
    anim_started: bool,
}

/// Composant actif pendant le pattern "full_auto".
/// La tourelle balaie de gauche à droite en tirant à intervalle régulier.
/// La vitesse de balayage et la cadence de tir augmentent avec le temps.
#[derive(Component)]
struct GatlingFullAuto {
    /// Angle relatif courant (radians, par rapport au repos).
    current_angle: f32,
    /// Direction du balayage : +1.0 ou -1.0.
    sweep_dir: f32,
    /// Temps écoulé depuis le début du pattern.
    elapsed: f32,
    /// Durée totale du pattern (secondes).
    duration: f32,
    /// Timer pour le prochain tir.
    fire_timer: Timer,
    /// Frame courante de l'animation de tir (None = pas d'anim en cours).
    anim_frame: Option<usize>,
    /// Timer de l'animation de tir.
    anim_timer: Timer,
}

/// Stocke le `EntryEdge` du Mothership parent pour calculer l'angle de base.
/// Les Gatlings standalone utilisent `Top` par défaut.
#[derive(Component)]
struct GatlingBaseEdge(EntryEdge);

/// Marqueur pour le Mothership.
#[derive(Component)]
pub struct MothershipMarker;

/// État et données du Mothership.
#[derive(Component)]
pub struct Mothership {
    pub state: MothershipPhase,
    /// Si true, le Mothership peut être endommagé (réservé pour la suite).
    pub vulnerable: bool,
    /// Bord d'apparition — détermine la direction d'entrée, de fuite et la disposition.
    pub edge: EntryEdge,
    /// Timer de la phase Entering.
    pub anim_timer: Timer,
    /// Position de départ hors écran (pour l'animation Entering).
    pub start_pos: Vec2,
    /// Position de repos après Entering (ancre pour le flottement).
    pub anchor_pos: Vec2,
    /// Temps accumulé pour le flottement sinusoïdal.
    pub drift_time: f32,
    /// Entités Gatling rattachées.
    pub gatlings: Vec<Entity>,
}

/// Phases du Mothership.
#[derive(PartialEq, Eq)]
pub enum MothershipPhase {
    /// Se déplace vers l'intérieur de l'écran pendant 3 secondes.
    Entering,
    /// Immobile, attend que les Gatlings meurent.
    Active,
    /// Recule doucement hors de l'écran vers le bord d'origine.
    Dying,
}

// ─── Ressources ─────────────────────────────────────────────────────

/// Frames préchargées de la Gatling (dossier images/gatling/).
#[derive(Resource)]
struct GatlingFrames(Vec<Handle<Image>>);

// ─── Préchargement ─────────────────────────────────────────────────

fn preload_gatling_frames(mut commands: Commands, asset_server: Res<AssetServer>) {
    let frames = load_frames_from_folder(&asset_server, "images/gatling")
        .expect("gatling frames folder missing or empty");
    commands.insert_resource(GatlingFrames(frames));
}

// ─── Offsets Gatling de base (convention Top) ───────────────────────

/// Calcule les offsets Gatling de base (convention Top).
/// `x` = spread horizontal, `y` = profondeur sous le Mothership.
/// Avec l'anchor au sommet du sprite (0, 0.5), la translation = le haut du sprite.
/// On place le haut du sprite au bord inférieur du Mothership.
fn base_gatling_offsets() -> [Vec2; 3] {
    let depth_y = -(MOTHERSHIP_BASE_SIZE.y / 2.0);
    [
        Vec2::new(-GATLING_SPACING, depth_y),
        Vec2::new(0.0, depth_y),
        Vec2::new(GATLING_SPACING, depth_y),
    ]
}

/// Étendue totale d'une Gatling depuis le centre du Mothership
/// dans la direction d'entrée (convention Top = vers le bas).
/// Avec l'anchor au sommet : translation au bord du Mothership + sprite_size complet.
fn gatling_total_extent() -> f32 {
    MOTHERSHIP_BASE_SIZE.y / 2.0 + GATLING_SPRITE_SIZE
}

// ─── Spawn Mothership (via spawn_requests "mothership") ─────────────

/// Consomme les configs dans `MothershipSpawnQueue`.
/// Spawne un Mothership (placeholder visuel) + 3 Gatlings liées,
/// chaque tourelle avec sa propre séquence de patterns.
fn spawn_mothership_oneshot(
    mut commands: Commands,
    mut spawn_queue: ResMut<MothershipSpawnQueue>,
    frames: Res<GatlingFrames>,
    windows: Query<&Window>,
) {
    if spawn_queue.0.is_empty() {
        return;
    }
    let config = spawn_queue.0.remove(0);

    let edge = EntryEdge::from_spawn_position(config.edge);
    let window = windows.single();
    let half_w = window.width() / 2.0;
    let half_h = window.height() / 2.0;

    let extent = gatling_total_extent();
    let pos = edge.spawn_position(half_w, half_h, extent);
    let sprite_size = edge.sprite_size();
    let rotation = edge.sprite_rotation();

    // ─── Spawn du Mothership (placeholder) ───────────────────────
    let mothership_entity = commands
        .spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::rgba(0.2, 0.2, 0.3, 0.8),
                    custom_size: Some(sprite_size),
                    ..default()
                },
                transform: Transform::from_xyz(pos.x, pos.y, 0.4),
                ..default()
            },
            MothershipMarker,
        ))
        .id();

    // ─── Spawn des 3 Gatlings ────────────────────────────────────
    let base_offsets = base_gatling_offsets();
    let mut gatling_entities = Vec::with_capacity(3);

    for (i, base_offset) in base_offsets.iter().enumerate() {
        let offset = edge.transform_offset(*base_offset);
        let gatling_pos = Vec2::new(pos.x + offset.x, pos.y + offset.y);
        let phase = &GATLING.phases[0];
        let first_frame = frames.0.first().cloned().unwrap_or_default();

        // Récupérer la config de cette tourelle (ou la dernière si pas assez de configs)
        let turret_config = if config.turrets.is_empty() {
            None
        } else {
            Some(config.turrets[i.min(config.turrets.len() - 1)].clone())
        };

        let mut entity_cmds = commands.spawn((
            SpriteBundle {
                texture: first_frame,
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(GATLING_SPRITE_SIZE)),
                    anchor: edge.gatling_anchor(),
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
                health: phase.health,
                max_health: phase.health,
                state: EnemyState::Entering,
                radius: GATLING.radius,
                sprite_size: GATLING.sprite_size,
                anim_timer: Timer::from_seconds(GATLING_ENTERING_DURATION, TimerMode::Once),
                phases: GATLING.phases,
                death_duration: GATLING.death_duration,
                death_shake_max: GATLING.death_shake_max,
                hit_sound: GATLING.hit_sound,
                death_explosion_sound: GATLING.death_explosion_sound,
                hit_flash_color: None,
            },
            GatlingMarker,
            GatlingBaseEdge(edge),
            MothershipLink {
                mothership: mothership_entity,
                offset,
            },
            GatlingEnteringAnim {
                timer: Timer::from_seconds(GATLING_ANIM_INTERVAL, TimerMode::Repeating),
                current_frame: 0,
            },
            PatternIndex(0),
            PatternTimer(Timer::from_seconds(0.0, TimerMode::Once)),
            DropTable {
                drops: &GATLING_DROP_TABLE,
            },
        ));

        // Insérer l'override de patterns si une config tourelle est fournie
        if let Some(tc) = turret_config {
            entity_cmds.insert(GatlingPatternOverride {
                patterns: tc.patterns,
            });
        }

        let gatling_entity = entity_cmds.id();

        gatling_entities.push(gatling_entity);
    }

    // ─── Insérer le composant Mothership avec la liste des gatlings ─
    // L'ancre = position finale après Entering.
    let dir = edge.enter_direction();
    let anchor = pos + dir * GATLING_ENTERING_DISTANCE;
    commands.entity(mothership_entity).insert(Mothership {
        state: MothershipPhase::Entering,
        vulnerable: false,
        edge,
        anim_timer: Timer::from_seconds(GATLING_ENTERING_DURATION, TimerMode::Once),
        start_pos: pos,
        anchor_pos: anchor,
        drift_time: 0.0,
        gatlings: gatling_entities,
    });
}

// ─── Spawn Gatling standalone (via spawn_requests "gatling") ────────

/// Consomme les requêtes "gatling" dans `difficulty.spawn_requests`.
/// Spawne des Gatlings indépendantes (sans Mothership).
fn spawn_gatlings_oneshot(
    mut commands: Commands,
    mut difficulty: ResMut<crate::difficulty::Difficulty>,
    frames: Res<GatlingFrames>,
    windows: Query<&Window>,
) {
    let Some(pos_idx) = difficulty
        .spawn_requests
        .iter()
        .position(|(name, _, _)| *name == "gatling")
    else {
        return;
    };
    let (_name, count, spawn_pos) = difficulty.spawn_requests.remove(pos_idx);

    let window = windows.single();
    for _ in 0..count {
        let pos = spawn_pos.resolve(window, 60.0);
        let phase = &GATLING.phases[0];
        let first_frame = frames.0.first().cloned().unwrap_or_default();

        commands.spawn((
            SpriteBundle {
                texture: first_frame,
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(GATLING_SPRITE_SIZE)),
                    anchor: EntryEdge::Top.gatling_anchor(),
                    ..default()
                },
                transform: Transform::from_xyz(pos.x, pos.y, 0.5),
                ..default()
            },
            Enemy {
                health: phase.health,
                max_health: phase.health,
                state: EnemyState::Entering,
                radius: GATLING.radius,
                sprite_size: GATLING.sprite_size,
                anim_timer: Timer::from_seconds(GATLING_ENTERING_DURATION, TimerMode::Once),
                phases: GATLING.phases,
                death_duration: GATLING.death_duration,
                death_shake_max: GATLING.death_shake_max,
                hit_sound: GATLING.hit_sound,
                death_explosion_sound: GATLING.death_explosion_sound,
                hit_flash_color: None,
            },
            GatlingMarker,
            GatlingBaseEdge(EntryEdge::Top),
            GatlingStartY(pos.y),
            GatlingEnteringAnim {
                timer: Timer::from_seconds(GATLING_ANIM_INTERVAL, TimerMode::Repeating),
                current_frame: 0,
            },
            PatternIndex(0),
            PatternTimer(Timer::from_seconds(0.0, TimerMode::Once)),
            DropTable {
                drops: &GATLING_DROP_TABLE,
            },
        ));
    }
}

// ─── Mothership Entering : avance vers l'intérieur de l'écran ───────

fn mothership_entering(
    time: Res<Time>,
    mut mothership_q: Query<(&mut Mothership, &mut Transform), With<MothershipMarker>>,
    mut enemy_q: Query<&mut Enemy, With<GatlingMarker>>,
) {
    for (mut mothership, mut transform) in mothership_q.iter_mut() {
        if mothership.state != MothershipPhase::Entering {
            continue;
        }

        mothership.anim_timer.tick(time.delta());
        let progress = mothership.anim_timer.fraction();

        // Ease-out quadratique
        let eased = 1.0 - (1.0 - progress).powi(2);
        let dir = mothership.edge.enter_direction();
        let displacement = dir * GATLING_ENTERING_DISTANCE * eased;
        transform.translation.x = mothership.start_pos.x + displacement.x;
        transform.translation.y = mothership.start_pos.y + displacement.y;

        if mothership.anim_timer.finished() {
            let final_disp = dir * GATLING_ENTERING_DISTANCE;
            transform.translation.x = mothership.start_pos.x + final_disp.x;
            transform.translation.y = mothership.start_pos.y + final_disp.y;
            mothership.state = MothershipPhase::Active;

            // Transitionner toutes les Gatlings en Active(0)
            for &gatling_entity in &mothership.gatlings {
                if let Ok(mut enemy) = enemy_q.get_mut(gatling_entity) {
                    let phase_def = &enemy.phases[0];
                    enemy.state = EnemyState::Active(0);
                    enemy.health = phase_def.health;
                    enemy.max_health = phase_def.health;
                }
            }
        }
    }
}

// ─── Flottement du Mothership pendant Active ───────────────────────

/// Flottement sinusoïdal du Mothership en phase Active.
/// Axe principal (grande amplitude) = perpendiculaire au bord d'entrée.
/// Axe secondaire (petite amplitude) = parallèle au bord d'entrée.
fn mothership_drift(
    time: Res<Time>,
    mut query: Query<(&mut Mothership, &mut Transform), With<MothershipMarker>>,
) {
    for (mut mothership, mut transform) in query.iter_mut() {
        if mothership.state != MothershipPhase::Active {
            continue;
        }

        mothership.drift_time += time.delta_seconds();
        let t = mothership.drift_time;

        // Axe principal = perpendiculaire au bord d'entrée (gauche-droite pour Top/Bottom)
        // Axe secondaire = parallèle (haut-bas pour Top/Bottom)
        let main_offset = (t * MOTHERSHIP_DRIFT_MAIN_FREQ).sin() * MOTHERSHIP_DRIFT_MAIN_AMP;
        let minor_offset = (t * MOTHERSHIP_DRIFT_MINOR_FREQ).cos() * MOTHERSHIP_DRIFT_MINOR_AMP;

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

// ─── Synchronisation positions Gatling → Mothership ─────────────────

fn mothership_sync_positions(
    mothership_q: Query<&Transform, (With<MothershipMarker>, Without<GatlingMarker>)>,
    mut gatling_q: Query<
        (&MothershipLink, &Enemy, &mut Transform),
        (With<GatlingMarker>, Without<MothershipMarker>),
    >,
) {
    for (link, enemy, mut transform) in gatling_q.iter_mut() {
        // Ne pas synchroniser pendant la mort (la gatling joue sa propre animation)
        if matches!(enemy.state, EnemyState::Dying | EnemyState::Dead) {
            continue;
        }
        if let Ok(ms_transform) = mothership_q.get(link.mothership) {
            transform.translation.x = ms_transform.translation.x + link.offset.x;
            transform.translation.y = ms_transform.translation.y + link.offset.y;
        }
    }
}

// ─── Détection de mort du Mothership ────────────────────────────────

fn mothership_death_detection(
    mut mothership_q: Query<&mut Mothership, With<MothershipMarker>>,
    enemy_q: Query<&Enemy>,
) {
    for mut mothership in mothership_q.iter_mut() {
        if mothership.state != MothershipPhase::Active {
            continue;
        }

        let all_dead = mothership.gatlings.iter().all(|&e| match enemy_q.get(e) {
            Ok(enemy) => matches!(enemy.state, EnemyState::Dying | EnemyState::Dead),
            Err(_) => true, // entité despawnée = morte
        });

        if all_dead {
            mothership.state = MothershipPhase::Dying;
        }
    }
}

// ─── Mothership Dying : recule vers le bord d'origine ───────────────

fn mothership_dying(
    mut commands: Commands,
    time: Res<Time>,
    mut mothership_q: Query<
        (Entity, &Mothership, &mut Transform, &mut Sprite),
        With<MothershipMarker>,
    >,
    mut level_events: EventWriter<LevelActionEvent>,
    windows: Query<&Window>,
) {
    // Compter les Motherships encore en jeu AVANT la boucle (évite le conflit borrow).
    let total_alive: usize = mothership_q.iter().count();

    let window = windows.single();
    let half_w = window.width() / 2.0;
    let half_h = window.height() / 2.0;
    let dt = time.delta_seconds();

    let mut despawned_count = 0usize;

    for (entity, mothership, mut transform, mut sprite) in mothership_q.iter_mut() {
        if mothership.state != MothershipPhase::Dying {
            continue;
        }

        let exit_dir = mothership.edge.exit_direction();

        // Reculer vers le bord d'origine
        transform.translation.x += exit_dir.x * MOTHERSHIP_DYING_SPEED * dt;
        transform.translation.y += exit_dir.y * MOTHERSHIP_DYING_SPEED * dt;

        // Fade out progressif
        let delta = Vec2::new(
            transform.translation.x - mothership.start_pos.x,
            transform.translation.y - mothership.start_pos.y,
        );
        let dist_toward_exit = delta.dot(exit_dir).max(0.0);
        let fade_range = GATLING_ENTERING_DISTANCE + gatling_total_extent();
        let progress = (dist_toward_exit / fade_range).clamp(0.0, 1.0);
        sprite.color.set_a(0.8 * (1.0 - progress));

        // Hors de l'écran → despawn
        if mothership.edge.is_offscreen(transform.translation, half_w, half_h) {
            if let Some(e) = commands.get_entity(entity) {
                e.despawn_recursive();
            }
            despawned_count += 1;
        }
    }

    // MarkLevelComplete seulement quand le dernier Mothership vient de sortir
    if despawned_count > 0 && despawned_count >= total_alive {
        level_events.send(LevelActionEvent(vec![Action::MarkLevelComplete]));
    }
}

// ─── Gatling standalone Entering (sans Mothership) ──────────────────

/// Descente d'un Gatling standalone (sans Mothership).
/// Les Gatlings liées à un Mothership sont positionnées par `mothership_sync_positions`.
fn gatling_standalone_entering(
    time: Res<Time>,
    mut query: Query<
        (&mut Enemy, &mut Transform, &GatlingStartY),
        (With<GatlingMarker>, Without<MothershipLink>),
    >,
) {
    for (mut enemy, mut transform, start_y) in query.iter_mut() {
        if enemy.state != EnemyState::Entering {
            continue;
        }

        enemy.anim_timer.tick(time.delta());
        let progress = enemy.anim_timer.fraction();

        // Ease-out quadratique
        let eased = 1.0 - (1.0 - progress).powi(2);
        transform.translation.y = start_y.0 - GATLING_ENTERING_DISTANCE * eased;

        if enemy.anim_timer.finished() {
            transform.translation.y = start_y.0 - GATLING_ENTERING_DISTANCE;
            enemy.state = EnemyState::Active(0);
        }
    }
}

// ─── Animation pendant Entering ─────────────────────────────────────

fn gatling_entering_animate(
    time: Res<Time>,
    frames: Res<GatlingFrames>,
    mut query: Query<(&Enemy, &mut Handle<Image>, &mut GatlingEnteringAnim), With<GatlingMarker>>,
) {
    for (enemy, mut texture, mut anim) in query.iter_mut() {
        if enemy.state != EnemyState::Entering {
            continue;
        }

        anim.timer.tick(time.delta());
        if anim.timer.just_finished() {
            anim.current_frame = (anim.current_frame + 1) % frames.0.len();
            *texture = frames.0[anim.current_frame].clone();
        }
    }
}

// ─── Pattern executor ───────────────────────────────────────────────

/// Cycle les patterns de la Gatling. Quand "shoot" se déclenche :
/// calcule la direction vers le joueur, insère le composant `GatlingShoot`.
/// Quand "idle" se déclenche : retire `GatlingShoot`, reset la rotation.
fn gatling_pattern_executor(
    time: Res<Time>,
    mut commands: Commands,
    frames: Res<GatlingFrames>,
    mut gatling_q: Query<
        (
            Entity,
            &Enemy,
            &Transform,
            &GatlingBaseEdge,
            &mut PatternTimer,
            &mut PatternIndex,
            &mut Handle<Image>,
            Option<&GatlingPatternOverride>,
            Option<&GatlingShoot>,
            Option<&GatlingFullAuto>,
        ),
        With<GatlingMarker>,
    >,
    player_q: Query<&Transform, (With<Player>, Without<GatlingMarker>)>,
) {
    for (entity, enemy, transform, base_edge, mut pattern_timer, mut pat_idx, mut texture, override_opt, shoot_opt, full_auto_opt) in
        gatling_q.iter_mut()
    {
        let phase_idx = match &enemy.state {
            EnemyState::Active(idx) => *idx,
            _ => continue,
        };

        pattern_timer.0.tick(time.delta());
        if !pattern_timer.0.just_finished() {
            continue;
        }

        // Utiliser l'override de patterns si présent, sinon les patterns de la phase
        let (pattern_name, pattern_duration, pattern_count) = if let Some(ov) = override_opt {
            if ov.patterns.is_empty() {
                continue;
            }
            let idx = pat_idx.0 % ov.patterns.len();
            let p = &ov.patterns[idx];
            (p.name, p.duration, ov.patterns.len())
        } else {
            let phase = &enemy.phases[phase_idx];
            if phase.patterns.is_empty() {
                continue;
            }
            let idx = pat_idx.0 % phase.patterns.len();
            let p = &phase.patterns[idx];
            (p.name, p.duration, phase.patterns.len())
        };

        pat_idx.0 += 1;

        // Programmer le timer pour le prochain pattern
        let next_duration = if let Some(ov) = override_opt {
            ov.patterns[pat_idx.0 % ov.patterns.len()].duration
        } else {
            let phase = &enemy.phases[phase_idx];
            phase.patterns[pat_idx.0 % phase.patterns.len()].duration
        };
        pattern_timer.0 = Timer::from_seconds(next_duration, TimerMode::Once);

        // Récupérer l'angle courant de la tourelle (depuis le pattern actif)
        let prev_angle = if let Some(s) = shoot_opt {
            s.current_angle
        } else if let Some(fa) = full_auto_opt {
            fa.current_angle
        } else {
            0.0
        };

        match pattern_name {
            "aim_and_shoot" => {
                commands.entity(entity).remove::<GatlingFullAuto>();

                // Direction de base du canon (repos) = direction d'entrée du Mothership
                let cannon_dir = base_edge.0.enter_direction();
                let base_angle = cannon_dir.y.atan2(cannon_dir.x);

                // Direction vers le joueur
                let aim_dir = if let Ok(player_transform) = player_q.get_single() {
                    let diff = player_transform.translation.truncate()
                        - transform.translation.truncate();
                    if diff.length_squared() > 0.01 {
                        diff.normalize()
                    } else {
                        cannon_dir
                    }
                } else {
                    cannon_dir
                };

                let aim_angle = aim_dir.y.atan2(aim_dir.x);
                let mut relative_angle = aim_angle - base_angle;
                while relative_angle > std::f32::consts::PI {
                    relative_angle -= std::f32::consts::TAU;
                }
                while relative_angle < -std::f32::consts::PI {
                    relative_angle += std::f32::consts::TAU;
                }

                let max_rad = GATLING_AIM_MAX_ANGLE.to_radians();
                let clamped_angle = relative_angle.clamp(-max_rad, max_rad);

                commands.entity(entity).insert(GatlingShoot {
                    target_angle: clamped_angle,
                    current_angle: prev_angle,
                    elapsed: 0.0,
                    duration: pattern_duration,
                    anim_timer: Timer::from_seconds(GATLING_SHOOT_ANIM_INTERVAL, TimerMode::Repeating),
                    current_frame: 0,
                    fired: false,
                    anim_started: false,
                });
            }
            "full_auto" => {
                commands.entity(entity).remove::<GatlingShoot>();

                commands.entity(entity).insert(GatlingFullAuto {
                    current_angle: prev_angle,
                    sweep_dir: 1.0,
                    elapsed: 0.0,
                    duration: pattern_duration,
                    fire_timer: Timer::from_seconds(FULL_AUTO_FIRE_INTERVAL_START, TimerMode::Repeating),
                    anim_frame: None,
                    anim_timer: Timer::from_seconds(FULL_AUTO_SHOOT_ANIM_INTERVAL, TimerMode::Repeating),
                });
            }
            "idle" => {
                commands.entity(entity).remove::<GatlingShoot>();
                commands.entity(entity).remove::<GatlingFullAuto>();
            }
            _ => {}
        }
    }
}

// ─── Mise à jour du pattern shoot ───────────────────────────────────

/// Gère la rotation vers le joueur, l'animation de tir, et le spawn du projectile.
fn gatling_shoot_update(
    time: Res<Time>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    frames: Res<GatlingFrames>,
    mut query: Query<
        (
            Entity,
            &GatlingBaseEdge,
            &mut GatlingShoot,
            &mut Transform,
            &mut Handle<Image>,
        ),
        With<GatlingMarker>,
    >,
    player_q: Query<&Transform, (With<Player>, Without<GatlingMarker>)>,
) {
    let dt = time.delta_seconds();
    let max_rad = GATLING_AIM_MAX_ANGLE.to_radians();

    for (_entity, base_edge, mut shoot, mut transform, mut texture) in query.iter_mut() {
        let rest_rot = base_edge.0.sprite_rotation();
        shoot.elapsed += dt;

        // Durée de l'animation de tir
        let anim_total_duration = frames.0.len() as f32 * GATLING_SHOOT_ANIM_INTERVAL;
        // L'animation démarre quand il reste juste assez de temps dans le pattern
        let anim_start_time = shoot.duration - anim_total_duration;

        // ── Suivi continu du joueur (même pendant l'animation de tir) ──
        {
            let cannon_dir = base_edge.0.enter_direction();
            let base_atan2 = cannon_dir.y.atan2(cannon_dir.x);

            if let Ok(player_transform) = player_q.get_single() {
                let diff = player_transform.translation.truncate()
                    - transform.translation.truncate();
                if diff.length_squared() > 0.01 {
                    let aim_atan2 = diff.y.atan2(diff.x);
                    let mut relative = aim_atan2 - base_atan2;
                    while relative > std::f32::consts::PI { relative -= std::f32::consts::TAU; }
                    while relative < -std::f32::consts::PI { relative += std::f32::consts::TAU; }
                    shoot.target_angle = relative.clamp(-max_rad, max_rad);
                }
            }

            // Rotation progressive vers la cible
            let speed_rad = GATLING_AIM_SPEED.to_radians() * dt;
            let angle_diff = shoot.target_angle - shoot.current_angle;
            let step = angle_diff.clamp(-speed_rad, speed_rad);
            shoot.current_angle += step;
            transform.rotation = rest_rot * Quat::from_rotation_z(shoot.current_angle);
        }

        // ── Déclencher l'animation de tir quand le temps est venu ──
        if !shoot.anim_started {
            if shoot.elapsed >= anim_start_time {
                shoot.anim_started = true;
                shoot.current_frame = 0;
                shoot.anim_timer = Timer::from_seconds(GATLING_SHOOT_ANIM_INTERVAL, TimerMode::Repeating);
            }
        } else {
            // Animation de tir
            shoot.anim_timer.tick(time.delta());

            if shoot.anim_timer.just_finished() {
                shoot.current_frame += 1;
                if shoot.current_frame < frames.0.len() {
                    *texture = frames.0[shoot.current_frame].clone();
                }
            }

            // Tirer le projectile à mi-animation
            let fire_frame = frames.0.len() / 2;
            if !shoot.fired && shoot.current_frame >= fire_frame {
                shoot.fired = true;

                // Direction du tir : rotation totale appliquée au vecteur local "vers le bas"
                // (le canon pointe dans la direction d'entrée, soit -Y en espace local du sprite)
                let total_rot = rest_rot * Quat::from_rotation_z(shoot.current_angle);
                let local_cannon = Vec3::new(0.0, -1.0, 0.0);
                let shoot_dir_3 = total_rot.mul_vec3(local_cannon);
                let shoot_dir = Vec2::new(shoot_dir_3.x, shoot_dir_3.y);

                // Bout du canon : anchor au sommet, le sprite s'étend de SPRITE_SIZE
                // dans la direction du canon
                let cannon_tip = transform.translation.truncate()
                    + shoot_dir * GATLING_SPRITE_SIZE;

                commands.spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            color: Color::rgba(1.0, 0.3, 0.3, 1.0),
                            custom_size: Some(Vec2::splat(GATLING_PROJECTILE_RADIUS * 2.0)),
                            ..default()
                        },
                        transform: Transform::from_xyz(cannon_tip.x, cannon_tip.y, 0.6),
                        ..default()
                    },
                    EnemyProjectile {
                        velocity: Vec3::new(
                            shoot_dir.x * GATLING_PROJECTILE_SPEED,
                            shoot_dir.y * GATLING_PROJECTILE_SPEED,
                            0.0,
                        ),
                        radius: GATLING_PROJECTILE_RADIUS,
                    },
                ));

                // Son de tir
                commands.spawn(AudioBundle {
                    source: asset_server.load("audio/sfx/gatling_shoot.ogg"),
                    settings: PlaybackSettings::DESPAWN,
                });
            }

            // Fin de l'animation → garder la rotation, reset le sprite au repos
            if shoot.current_frame >= frames.0.len() {
                if let Some(frame) = frames.0.first() {
                    *texture = frame.clone();
                }
            }
        }
    }
}

// ─── Mise à jour du pattern full_auto ───────────────────────────────

/// Balayage automatique gauche↔droite avec tir à cadence croissante.
fn gatling_full_auto_update(
    time: Res<Time>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    frames: Res<GatlingFrames>,
    mut query: Query<
        (
            &GatlingBaseEdge,
            &mut GatlingFullAuto,
            &mut Transform,
            &mut Handle<Image>,
        ),
        With<GatlingMarker>,
    >,
) {
    let dt = time.delta_seconds();
    let max_rad = GATLING_AIM_MAX_ANGLE.to_radians();

    for (base_edge, mut auto, mut transform, mut texture) in query.iter_mut() {
        let rest_rot = base_edge.0.sprite_rotation();
        auto.elapsed += dt;

        // ── Courbe d'accélération ──
        let progress = (auto.elapsed / auto.duration).min(1.0);
        let ramp = progress.powf(FULL_AUTO_RAMP_FACTOR);

        // Vitesse de balayage interpolée
        let sweep_speed = FULL_AUTO_SWEEP_SPEED_START
            + (FULL_AUTO_SWEEP_SPEED_MAX - FULL_AUTO_SWEEP_SPEED_START) * ramp;
        let sweep_rad = sweep_speed.to_radians() * dt;

        // Intervalle de tir interpolé
        let fire_interval = FULL_AUTO_FIRE_INTERVAL_START
            + (FULL_AUTO_FIRE_INTERVAL_MIN - FULL_AUTO_FIRE_INTERVAL_START) * ramp;
        auto.fire_timer.set_duration(std::time::Duration::from_secs_f32(fire_interval.max(0.05)));

        // ── Balayage ping-pong ──
        auto.current_angle += auto.sweep_dir * sweep_rad;
        if auto.current_angle >= max_rad {
            auto.current_angle = max_rad;
            auto.sweep_dir = -1.0;
        } else if auto.current_angle <= -max_rad {
            auto.current_angle = -max_rad;
            auto.sweep_dir = 1.0;
        }

        // Appliquer la rotation
        transform.rotation = rest_rot * Quat::from_rotation_z(auto.current_angle);

        // ── Animation de tir en cours ──
        if let Some(frame_idx) = auto.anim_frame {
            auto.anim_timer.tick(time.delta());
            if auto.anim_timer.just_finished() {
                let next = frame_idx + 1;
                if next < frames.0.len() {
                    auto.anim_frame = Some(next);
                    *texture = frames.0[next].clone();
                } else {
                    // Fin de l'animation → retour au sprite de repos
                    auto.anim_frame = None;
                    if let Some(f) = frames.0.first() {
                        *texture = f.clone();
                    }
                }
            }
        }

        // ── Tir ──
        auto.fire_timer.tick(time.delta());
        if auto.fire_timer.just_finished() {
            // Lancer l'animation de tir
            auto.anim_frame = Some(0);
            auto.anim_timer = Timer::from_seconds(FULL_AUTO_SHOOT_ANIM_INTERVAL, TimerMode::Repeating);
            if let Some(f) = frames.0.first() {
                *texture = f.clone();
            }

            // Direction du tir = direction actuelle du canon
            let total_rot = rest_rot * Quat::from_rotation_z(auto.current_angle);
            let local_cannon = Vec3::new(0.0, -1.0, 0.0);
            let shoot_dir_3 = total_rot.mul_vec3(local_cannon);
            let shoot_dir = Vec2::new(shoot_dir_3.x, shoot_dir_3.y);

            // Bout du canon
            let cannon_tip = transform.translation.truncate()
                + shoot_dir * GATLING_SPRITE_SIZE;

            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: Color::rgba(1.0, 0.3, 0.3, 1.0),
                        custom_size: Some(Vec2::splat(GATLING_PROJECTILE_RADIUS * 2.0)),
                        ..default()
                    },
                    transform: Transform::from_xyz(cannon_tip.x, cannon_tip.y, 0.6),
                    ..default()
                },
                EnemyProjectile {
                    velocity: Vec3::new(
                        shoot_dir.x * GATLING_PROJECTILE_SPEED,
                        shoot_dir.y * GATLING_PROJECTILE_SPEED,
                        0.0,
                    ),
                    radius: GATLING_PROJECTILE_RADIUS,
                },
            ));

            // Son de tir
            commands.spawn(AudioBundle {
                source: asset_server.load("audio/sfx/gatling_shoot.ogg"),
                settings: PlaybackSettings::DESPAWN,
            });
        }
    }
}
