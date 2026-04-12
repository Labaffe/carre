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
use crate::enemy::{Enemy, EnemyState, PatternIndex, PatternTimer};
use crate::explosion::load_frames_from_folder;
use crate::item::{DropTable, ItemType};
use crate::level::{Action, LevelActionEvent};
use crate::pause::not_paused;
use crate::state::GameState;
use bevy::prelude::*;

pub struct GatlingPlugin;

impl Plugin for GatlingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, preload_gatling_frames)
            .add_systems(
                Update,
                (
                    spawn_mothership_oneshot,
                    spawn_gatlings_oneshot,
                    mothership_entering,
                    mothership_sync_positions,
                    mothership_death_detection,
                    mothership_dying,
                    gatling_standalone_entering,
                    gatling_entering_animate,
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
/// Taille de base du placeholder Mothership (7×2 tiles de 128px).
/// Pour Top/Bottom : largeur × hauteur. Pour Left/Right : inversé automatiquement.
const MOTHERSHIP_BASE_SIZE: Vec2 = Vec2::new(896.0, 256.0);

/// Drop table : 10% bombe, 15% bonus score.
static GATLING_DROP_TABLE: [(ItemType, f32); 2] =
    [(ItemType::Bomb, 0.10), (ItemType::BonusScore, 0.15)];

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
    /// Position de départ (pour l'animation Entering).
    pub start_pos: Vec2,
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
fn base_gatling_offsets() -> [Vec2; 3] {
    let depth_y =
        -(MOTHERSHIP_BASE_SIZE.y / 2.0) - (GATLING_SPRITE_SIZE / 2.0) + (GATLING_SPRITE_SIZE / 4.0);
    [
        Vec2::new(-GATLING_SPACING, depth_y),
        Vec2::new(0.0, depth_y),
        Vec2::new(GATLING_SPACING, depth_y),
    ]
}

/// Étendue totale d'une Gatling depuis le centre du Mothership
/// dans la direction d'entrée (convention Top = vers le bas).
fn gatling_total_extent() -> f32 {
    let depth_y =
        (MOTHERSHIP_BASE_SIZE.y / 2.0) + (GATLING_SPRITE_SIZE / 2.0) - (GATLING_SPRITE_SIZE / 4.0);
    depth_y + GATLING_SPRITE_SIZE / 2.0
}

// ─── Spawn Mothership (via spawn_requests "mothership") ─────────────

/// Consomme les requêtes "mothership" dans `difficulty.spawn_requests`.
/// Spawne un Mothership (placeholder visuel) + 3 Gatlings liées.
/// Le bord d'apparition est déduit du `SpawnPosition` de la requête.
fn spawn_mothership_oneshot(
    mut commands: Commands,
    mut difficulty: ResMut<crate::difficulty::Difficulty>,
    frames: Res<GatlingFrames>,
    windows: Query<&Window>,
) {
    let Some(pos_idx) = difficulty
        .spawn_requests
        .iter()
        .position(|(name, _, _)| *name == "mothership")
    else {
        return;
    };
    let (_name, _count, spawn_pos) = difficulty.spawn_requests.remove(pos_idx);

    let edge = EntryEdge::from_spawn_position(spawn_pos);
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

    for base_offset in &base_offsets {
        let offset = edge.transform_offset(*base_offset);
        let gatling_pos = Vec2::new(pos.x + offset.x, pos.y + offset.y);
        let phase = &GATLING.phases[0];
        let first_frame = frames.0.first().cloned().unwrap_or_default();

        let gatling_entity = commands
            .spawn((
                SpriteBundle {
                    texture: first_frame,
                    sprite: Sprite {
                        custom_size: Some(Vec2::splat(GATLING_SPRITE_SIZE)),
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
            ))
            .id();

        gatling_entities.push(gatling_entity);
    }

    // ─── Insérer le composant Mothership avec la liste des gatlings ─
    commands.entity(mothership_entity).insert(Mothership {
        state: MothershipPhase::Entering,
        vulnerable: false,
        edge,
        anim_timer: Timer::from_seconds(GATLING_ENTERING_DURATION, TimerMode::Once),
        start_pos: pos,
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
