//! GreenUFO — ennemi simple utilisant le framework `enemy.rs`.
//!
//! Démontre l'utilisation minimale du framework : pas d'intro, pas de flexing,
//! pas d'idle. Spawne directement en `Active(0)` et meurt instantanément
//! comme un astéroïde (explosion + despawn).
//!
//! Patterns : rush (fonce vers le joueur 0.4s) → idle (pause 0.2s) → repeat
//! Son "green_ufo.ogg" au début de chaque rush.

use crate::enemies::GREEN_UFO;
use crate::enemy::{Enemy, EnemyState, PatternIndex, PatternTimer};
use crate::explosion::{load_frames_from_folder, spawn_custom_anim};
use crate::item::{DropTable, ItemType};
use crate::pause::not_paused;
use crate::player::Player;
use crate::state::GameState;
use bevy::prelude::*;

pub struct GreenUFOPlugin;

impl Plugin for GreenUFOPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GreenUFOSpawner {
            timer: Timer::from_seconds(GREEN_UFO_SPAWN_INTERVAL, TimerMode::Repeating),
        })
        .add_systems(Startup, preload_green_ufo_frames)
        .add_systems(
            Update,
            (
                spawn_green_ufos,
                spawn_green_ufos_oneshot,
                green_ufo_pattern_executor,
                green_ufo_rush_movement,
                green_ufo_animate,
                green_ufo_death,
            )
                .run_if(in_state(GameState::Playing))
                .run_if(not_paused),
        );
    }
}

// ─── Constantes ─────────────────────────────────────────────────────

/// Intervalle par défaut entre chaque vague (secondes).
const GREEN_UFO_SPAWN_INTERVAL: f32 = 2.0;
/// Vitesse du rush vers le joueur (px/s).
const GREEN_UFO_RUSH_SPEED: f32 = 800.0;
/// FPS de l'animation idle du GreenUFO.
const GREEN_UFO_ANIM_FPS: f32 = 12.0;
/// Marge par rapport aux bords de l'écran (px).
const GREEN_UFO_MARGIN: f32 = 40.0;

/// Drop table : 10% bombe, 15% bonus score.
static GREEN_UFO_DROP_TABLE: [(ItemType, f32); 2] =
    [(ItemType::Bomb, 0.10), (ItemType::BonusScore, 0.15)];

// ─── Composants ─────────────────────────────────────────────────────

/// Marqueur pour identifier les GreenUFO parmi les Enemy.
#[derive(Component)]
pub struct GreenUFOMarker;

/// Le GreenUFO est en train de foncer vers le joueur.
/// La direction est fixée au début du rush.
#[derive(Component)]
struct GreenUFORush {
    direction: Vec2,
}

/// Animation du GreenUFO (cycle de frames).
#[derive(Component)]
struct GreenUFOAnim {
    timer: Timer,
    current_frame: usize,
}

// ─── Ressources ─────────────────────────────────────────────────────

/// Frames préchargées du GreenUFO (idle).
#[derive(Resource)]
struct GreenUFOFrames(Vec<Handle<Image>>);

/// Frames préchargées de l'animation de mort du GreenUFO.
#[derive(Resource)]
struct GreenUFODeathFrames(Vec<Handle<Image>>);

#[derive(Resource)]
struct GreenUFOSpawner {
    timer: Timer,
}

// ─── Préchargement ──────────────────────────────────────────────────

fn preload_green_ufo_frames(mut commands: Commands, asset_server: Res<AssetServer>) {
    let frames = load_frames_from_folder(&asset_server, "images/green_ufo")
        .expect("green_ufo frames folder missing or empty");
    commands.insert_resource(GreenUFOFrames(frames));

    let death_frames = load_frames_from_folder(&asset_server, "images/green_ufo/death")
        .expect("green_ufo death frames folder missing or empty");
    commands.insert_resource(GreenUFODeathFrames(death_frames));
}

// ─── Spawn ──────────────────────────────────────────────────────────

fn spawn_green_ufos(
    mut commands: Commands,
    time: Res<Time>,
    mut spawner: ResMut<GreenUFOSpawner>,
    difficulty: Res<crate::difficulty::Difficulty>,
    frames: Res<GreenUFOFrames>,
    windows: Query<&Window>,
) {
    // Spawning contrôlé par le système de niveau via active_spawners
    let Some(&(wave_size, target_interval, spawn_pos)) = difficulty.active_spawners.get("green_ufo") else {
        return;
    };

    // Mettre à jour l'intervalle si le niveau l'a changé
    if (spawner.timer.duration().as_secs_f32() - target_interval).abs() > 0.01 {
        spawner.timer.set_duration(std::time::Duration::from_secs_f32(target_interval));
    }

    spawner.timer.tick(time.delta());
    if !spawner.timer.just_finished() {
        return;
    }

    let window = windows.single();
    for _ in 0..wave_size {
        spawn_one_green_ufo(&mut commands, &frames, window, spawn_pos);
    }
}

// ─── Spawn one-shot (via spawn_requests) ────────────────────────────

/// Consomme les requêtes "green_ufo" dans `difficulty.spawn_requests`
/// et spawne N GreenUFOs par requête.
fn spawn_green_ufos_oneshot(
    mut commands: Commands,
    mut difficulty: ResMut<crate::difficulty::Difficulty>,
    frames: Res<GreenUFOFrames>,
    windows: Query<&Window>,
) {
    let Some(pos) = difficulty
        .spawn_requests
        .iter()
        .position(|(name, _, _)| *name == "green_ufo")
    else {
        return;
    };
    let (_name, count, spawn_pos) = difficulty.spawn_requests.remove(pos);

    let window = windows.single();
    for _ in 0..count {
        spawn_one_green_ufo(&mut commands, &frames, window, spawn_pos);
    }
}

/// Spawne un seul GreenUFO à la position donnée.
fn spawn_one_green_ufo(
    commands: &mut Commands,
    frames: &GreenUFOFrames,
    window: &Window,
    spawn_pos: crate::difficulty::SpawnPosition,
) {
    let pos = spawn_pos.resolve(window, 60.0);

    let phase = &GREEN_UFO.phases[0];
    let first_frame = frames.0.first().cloned().unwrap_or_default();

    commands.spawn((
        SpriteBundle {
            texture: first_frame,
            sprite: Sprite {
                custom_size: Some(Vec2::splat(GREEN_UFO.sprite_size)),
                ..default()
            },
            transform: Transform::from_xyz(pos.x, pos.y, 0.5),
            ..default()
        },
        Enemy {
            health: phase.health,
            max_health: phase.health,
            state: EnemyState::Active(0),
            radius: GREEN_UFO.radius,
            sprite_size: GREEN_UFO.sprite_size,
            anim_timer: Timer::from_seconds(0.01, TimerMode::Once),
            phases: GREEN_UFO.phases,
            death_duration: GREEN_UFO.death_duration,
            death_shake_max: GREEN_UFO.death_shake_max,
            hit_sound: GREEN_UFO.hit_sound,
            death_explosion_sound: GREEN_UFO.death_explosion_sound,
        },
        GreenUFOMarker,
        GreenUFOAnim {
            timer: Timer::from_seconds(1.0 / GREEN_UFO_ANIM_FPS, TimerMode::Repeating),
            current_frame: 0,
        },
        PatternIndex(0),
        PatternTimer(Timer::from_seconds(0.0, TimerMode::Once)),
        DropTable {
            drops: &GREEN_UFO_DROP_TABLE,
        },
    ));
}

// ─── Pattern executor ───────────────────────────────────────────────

fn green_ufo_pattern_executor(
    time: Res<Time>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut ufo_q: Query<
        (
            Entity,
            &Enemy,
            &Transform,
            &mut PatternTimer,
            &mut PatternIndex,
        ),
        With<GreenUFOMarker>,
    >,
    player_q: Query<&Transform, (With<Player>, Without<GreenUFOMarker>)>,
) {
    for (entity, enemy, ufo_transform, mut pattern_timer, mut pat_idx) in ufo_q.iter_mut() {
        let phase_idx = match &enemy.state {
            EnemyState::Active(idx) => *idx,
            _ => continue,
        };

        pattern_timer.0.tick(time.delta());
        if !pattern_timer.0.just_finished() {
            continue;
        }

        let phase = &enemy.phases[phase_idx];
        if phase.patterns.is_empty() {
            continue;
        }

        let current_idx = pat_idx.0 % phase.patterns.len();
        let pattern = &phase.patterns[current_idx];
        pat_idx.0 += 1;

        // Programmer le timer pour le prochain pattern
        let next_idx = pat_idx.0 % phase.patterns.len();
        let next_duration = phase.patterns[next_idx].duration;
        pattern_timer.0 = Timer::from_seconds(next_duration, TimerMode::Once);

        match pattern.name {
            "rush" => {
                // Calculer la direction vers le joueur
                let direction = if let Ok(player_transform) = player_q.get_single() {
                    let diff = player_transform.translation.truncate()
                        - ufo_transform.translation.truncate();
                    diff.normalize_or_zero()
                } else {
                    Vec2::new(0.0, -1.0) // par défaut, foncer vers le bas
                };

                commands.entity(entity).insert(GreenUFORush { direction });
                // Son de rush
                commands.spawn(AudioBundle {
                    source: asset_server.load("audio/green_ufo.ogg"),
                    settings: PlaybackSettings {
                        volume: bevy::audio::Volume::new(0.8),
                        ..default()
                    },
                });
            }
            "idle" => {
                // Arrêter le rush
                commands.entity(entity).remove::<GreenUFORush>();
            }
            _ => {}
        }
    }
}

// ─── Mouvement rush ─────────────────────────────────────────────────

fn green_ufo_rush_movement(
    time: Res<Time>,
    mut commands: Commands,
    windows: Query<&Window>,
    mut query: Query<
        (
            Entity,
            &Enemy,
            &mut Transform,
            &GreenUFORush,
            &mut PatternTimer,
            &mut PatternIndex,
        ),
        With<GreenUFOMarker>,
    >,
) {
    let window = windows.single();
    let half_w = window.width() / 2.0 - GREEN_UFO_MARGIN;
    let half_h = window.height() / 2.0 - GREEN_UFO_MARGIN;
    let dt = time.delta_seconds();

    for (entity, enemy, mut transform, rush, mut pattern_timer, mut pat_idx) in query.iter_mut() {
        let phase_idx = match &enemy.state {
            EnemyState::Active(idx) => *idx,
            _ => continue,
        };

        transform.translation.x += rush.direction.x * GREEN_UFO_RUSH_SPEED * dt;
        transform.translation.y += rush.direction.y * GREEN_UFO_RUSH_SPEED * dt;

        // Détection collision avec les bords → changement de pattern
        let at_edge = transform.translation.x.abs() >= half_w
            || transform.translation.y.abs() >= half_h;

        if at_edge {
            // Clamper à l'intérieur
            transform.translation.x = transform.translation.x.clamp(-half_w, half_w);
            transform.translation.y = transform.translation.y.clamp(-half_h, half_h);

            // Retirer le rush et passer au pattern suivant (idle)
            commands.entity(entity).remove::<GreenUFORush>();

            let phase = &enemy.phases[phase_idx];
            if !phase.patterns.is_empty() {
                // Forcer le passage au prochain pattern "idle"
                // Trouver le prochain "idle" dans la liste
                pat_idx.0 += 1;
                let next_idx = pat_idx.0 % phase.patterns.len();
                let next_duration = phase.patterns[next_idx].duration;
                pattern_timer.0 = Timer::from_seconds(next_duration, TimerMode::Once);
            }
        }
    }
}

// ─── Animation ──────────────────────────────────────────────────────

fn green_ufo_animate(
    time: Res<Time>,
    frames: Res<GreenUFOFrames>,
    mut query: Query<(&mut Handle<Image>, &mut GreenUFOAnim), With<GreenUFOMarker>>,
) {
    for (mut texture, mut anim) in query.iter_mut() {
        anim.timer.tick(time.delta());
        if anim.timer.just_finished() {
            anim.current_frame = (anim.current_frame + 1) % frames.0.len();
            *texture = frames.0[anim.current_frame].clone();
        }
    }
}

// ─── Mort style astéroïde ───────────────────────────────────────────

fn green_ufo_death(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    death_frames: Res<GreenUFODeathFrames>,
    query: Query<(Entity, &Enemy, &Transform), (With<GreenUFOMarker>, Changed<Enemy>)>,
) {
    for (_entity, enemy, transform) in query.iter() {
        if enemy.state != EnemyState::Dying {
            continue;
        }

        // Animation de mort custom (frames green_ufo/death/)
        spawn_custom_anim(
            &mut commands,
            death_frames.0.clone(),
            transform.translation,
            Vec2::splat(GREEN_UFO.sprite_size),
            0.4,
        );

        commands.spawn(AudioBundle {
            source: asset_server.load("audio/green_ufo_death.ogg"),
            settings: PlaybackSettings::DESPAWN,
        });
    }
}
