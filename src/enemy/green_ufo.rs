//! GreenUFO — définition data-driven avec spawn system.
//!
//! Phases : `rush` (fonce vers le joueur) ↔ `idle` (pause) en boucle.
//! Mort = instantanée à PV=0 (phase `dying` → `dead` = DespawnSelf).

use std::time::Duration;

use bevy::prelude::*;

use crate::enemy::behaviors::{
    DespawnIfOffscreen, DespawnSelf, PlaySound, RushMove, SetRushDirection,
};
use crate::enemy::enemies::GREEN_UFO;
use crate::enemy::enemy::{Enemy, EnemyHitFlash};
use crate::enemy::system::{
    b, seq, EnemyDefinition, Noop, Phase, PhaseId, Transition, TransitionTrigger,
};
use crate::fx::explosion::{load_frames_from_folder, spawn_custom_anim};
use crate::game_manager::difficulty::Difficulty;
use crate::game_manager::state::GameState;
use crate::item::item::{DropTable, ItemType};
use crate::menu::pause::not_paused;
use crate::physic::health::Health;

const RUSH_SPEED: f32 = 800.0;
const RUSH_DURATION: f32 = 1.5;
const IDLE_DURATION: f32 = 1.0;
const GREEN_UFO_ANIM_FPS: f32 = 12.0;
const GREEN_UFO_SPAWN_INTERVAL: f32 = 2.0;

static GREEN_UFO_DROP_TABLE: [(ItemType, f32); 2] =
    [(ItemType::Bomb, 0.10), (ItemType::BonusScore, 0.15)];

// ─── Composants ─────────────────────────────────────────────────────

#[derive(Component)]
pub struct GreenUFOMarker;

#[derive(Component)]
struct GreenUFOAnim {
    timer: Timer,
    current_frame: usize,
}

#[derive(Resource)]
struct GreenUFOFrames(Vec<Handle<Image>>);

#[derive(Resource)]
struct GreenUFODeathFrames(Vec<Handle<Image>>);

#[derive(Resource)]
struct GreenUFOSpawner {
    timer: Timer,
}

// ─── Définition ─────────────────────────────────────────────────────

pub fn green_ufo_definition() -> EnemyDefinition {
    EnemyDefinition {
        name: "GreenUFO",
        initial_phase: PhaseId("rush"),
        phases: vec![
            Phase::new(
                PhaseId("rush"),
                seq(vec![
                    b(RushMove { speed: RUSH_SPEED }),
                    b(DespawnIfOffscreen { margin: 100.0 }),
                ]),
                vec![
                    Transition {
                        trigger: TransitionTrigger::Timer(Duration::from_secs_f32(RUSH_DURATION)),
                        target_phase: PhaseId("idle"),
                        priority: 0,
                    },
                    Transition {
                        trigger: TransitionTrigger::HealthBelow(0.01),
                        target_phase: PhaseId("dying"),
                        priority: 10,
                    },
                ],
            )
            .with_on_enter(seq(vec![
                b(SetRushDirection),
                b(PlaySound {
                    path: "audio/sfx/green_ufo.ogg",
                    volume: 0.8,
                }),
            ])),
            Phase::new(
                PhaseId("idle"),
                b(Noop),
                vec![
                    Transition {
                        trigger: TransitionTrigger::Timer(Duration::from_secs_f32(IDLE_DURATION)),
                        target_phase: PhaseId("rush"),
                        priority: 0,
                    },
                    Transition {
                        trigger: TransitionTrigger::HealthBelow(0.01),
                        target_phase: PhaseId("dying"),
                        priority: 10,
                    },
                ],
            ),
            Phase::new(PhaseId("dying"), b(Noop), vec![]).invulnerable(),
            Phase::new(PhaseId("dead"), b(DespawnSelf), vec![])
                .invulnerable(),
        ],
    }
}

// ─── Plugin ─────────────────────────────────────────────────────────

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
                animate_green_ufo,
                detect_death,
            )
                .run_if(in_state(GameState::Playing))
                .run_if(not_paused),
        );
    }
}

fn preload_green_ufo_frames(mut commands: Commands, asset_server: Res<AssetServer>) {
    let frames = load_frames_from_folder(&asset_server, "images/green_ufo")
        .expect("green_ufo frames folder missing or empty");
    commands.insert_resource(GreenUFOFrames(frames));

    let death_frames = load_frames_from_folder(&asset_server, "images/green_ufo/death")
        .expect("green_ufo death frames folder missing or empty");
    commands.insert_resource(GreenUFODeathFrames(death_frames));
}

fn spawn_green_ufos(
    mut commands: Commands,
    time: Res<Time>,
    mut spawner: ResMut<GreenUFOSpawner>,
    difficulty: Res<Difficulty>,
    frames: Res<GreenUFOFrames>,
    windows: Query<&Window>,
) {
    let Some(&(wave_size, target_interval, spawn_pos)) = difficulty.active_spawners.get("green_ufo")
    else {
        return;
    };

    if (spawner.timer.duration().as_secs_f32() - target_interval).abs() > 0.01 {
        spawner
            .timer
            .set_duration(std::time::Duration::from_secs_f32(target_interval));
    }

    spawner.timer.tick(time.delta());
    if !spawner.timer.just_finished() {
        return;
    }

    let window = windows.single();
    for _ in 0..wave_size {
        spawn_one(&mut commands, &frames, window, spawn_pos);
    }
}

fn spawn_green_ufos_oneshot(
    mut commands: Commands,
    mut difficulty: ResMut<Difficulty>,
    frames: Res<GreenUFOFrames>,
    windows: Query<&Window>,
) {
    let Some(pos) = difficulty
        .spawn_requests
        .iter()
        .position(|(n, _, _)| *n == "green_ufo")
    else {
        return;
    };
    let (_name, count, spawn_pos) = difficulty.spawn_requests.remove(pos);

    let window = windows.single();
    for _ in 0..count {
        spawn_one(&mut commands, &frames, window, spawn_pos);
    }
}

fn spawn_one(
    commands: &mut Commands,
    frames: &GreenUFOFrames,
    window: &Window,
    spawn_pos: crate::game_manager::difficulty::SpawnPosition,
) {
    let pos = spawn_pos.resolve(window, 60.0);
    let first_frame = frames.0.first().cloned().unwrap_or_default();

    commands.spawn((
        SpriteBundle {
            texture: first_frame,
            sprite: Sprite {
                custom_size: Some(Vec2::splat(GREEN_UFO.config.sprite_size)),
                ..default()
            },
            transform: Transform::from_xyz(pos.x, pos.y, 0.5),
            ..default()
        },
        Enemy::new(GREEN_UFO.config.to_config(), green_ufo_definition()),
        Health::new(GREEN_UFO.total_hp),
        GreenUFOMarker,
        GreenUFOAnim {
            timer: Timer::from_seconds(1.0 / GREEN_UFO_ANIM_FPS, TimerMode::Repeating),
            current_frame: 0,
        },
        DropTable {
            drops: &GREEN_UFO_DROP_TABLE,
        },
    ));
}

// ─── Animation idle ─────────────────────────────────────────────────

fn animate_green_ufo(
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

// ─── Mort : explosion style astéroïde ───────────────────────────────

/// Détecte l'entrée en phase `dying` et spawne l'explosion custom avant le
/// despawn (fait par le behavior `DespawnSelf` de la phase `dead`).
fn detect_death(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    death_frames: Res<GreenUFODeathFrames>,
    mut query: Query<
        (Entity, &Enemy, &Transform, &mut Visibility),
        (With<GreenUFOMarker>, Changed<Enemy>),
    >,
) {
    for (_entity, enemy, transform, mut visibility) in query.iter_mut() {
        if enemy.current_phase != PhaseId("dying") {
            continue;
        }
        *visibility = Visibility::Hidden;
        spawn_custom_anim(
            &mut commands,
            death_frames.0.clone(),
            transform.translation,
            Vec2::splat(GREEN_UFO.config.sprite_size),
            0.4,
        );
        commands.spawn(AudioBundle {
            source: asset_server.load("audio/sfx/green_ufo_death.ogg"),
            settings: PlaybackSettings::DESPAWN,
        });
    }
}
