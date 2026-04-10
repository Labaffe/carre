//! Joueur : spawn, mouvement ZQSD, rotation vers le réticule, animation par phase.
//!
//! Phase 1 (0–10s)  : image statique ship_0.png, vitesse 200, Standard Missile.
//! Phase 2 (10s+)   : animation phase_2/ (20 frames), vitesse 400, Red Projectile.
//! Phase 3 (boss rotation) : animation phase_3/ (9 frames), vitesse 800, Blue Projectiles.

use crate::crosshair::Crosshair;
use crate::difficulty::{BoomEvent, Difficulty};
use crate::explosion::load_frames_from_folder;
use crate::pause::PauseState;
use crate::state::GameState;
use crate::weapon::Weapon;
use bevy::prelude::*;

fn not_paused(pause: Res<PauseState>) -> bool {
    !pause.paused
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, preload_ship_textures)
            .add_systems(OnEnter(GameState::Playing), setup_player)
            .add_systems(
                Update,
                (
                    movement,
                    rotate_towards_crosshair,
                    update_player_phase,
                    animate_ship,
                    boom_flash_trigger,
                    boom_flash_update,
                )
                    .run_if(in_state(GameState::Playing))
                    .run_if(not_paused),
            );
    }
}

#[derive(Component)]
pub struct Player;

/// Flash blanc autour du vaisseau lors d'un boom.
const BOOM_FLASH_DURATION: f32 = 0.25;

#[derive(Component)]
struct BoomFlash(Timer);

// ─── Phases du joueur ──────────────────────────────────────────────

const PHASE_1_SPEED: f32 = 200.0;
const PHASE_2_SPEED: f32 = 400.0;
const PHASE_3_SPEED: f32 = 1000.0;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PlayerPhase {
    Phase1,
    Phase2,
    Phase3,
}

#[derive(Component)]
pub struct ShipPhase {
    pub phase: PlayerPhase,
    pub speed: f32,
    timer: Timer,
    current_frame: usize,
}

// ─── Textures préchargées ──────────────────────────────────────────

#[derive(Resource)]
struct ShipTextures {
    phase_1: Handle<Image>,
    phase_2: Vec<Handle<Image>>,
    phase_3: Vec<Handle<Image>>,
}

fn preload_ship_textures(mut commands: Commands, asset_server: Res<AssetServer>) {
    let phase_1 = asset_server.load("images/player_ship/ship_0.png");
    let phase_2 = load_frames_from_folder(&asset_server, "images/player_ship/phase_2")
        .expect("phase_2 folder missing or empty");
    let phase_3 = load_frames_from_folder(&asset_server, "images/player_ship/phase_3")
        .expect("phase_3 folder missing or empty");
    commands.insert_resource(ShipTextures {
        phase_1,
        phase_2,
        phase_3,
    });
}

fn setup_player(mut commands: Commands, asset_server: Res<AssetServer>, windows: Query<&Window>) {
    let window = windows.single();
    let half_h = window.height() / 2.0;
    spawn_player(&mut commands, &asset_server, -half_h * 0.5);
}

pub fn spawn_player(commands: &mut Commands, asset_server: &Res<AssetServer>, start_y: f32) {
    commands.spawn((
        SpriteBundle {
            texture: asset_server.load("images/player_ship/ship_0.png"),
            sprite: Sprite {
                custom_size: Some(Vec2::new(128.0, 128.0)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, start_y, 0.0),
            ..default()
        },
        Player,
        Weapon::default(),
        ShipPhase {
            phase: PlayerPhase::Phase1,
            speed: PHASE_1_SPEED,
            timer: Timer::from_seconds(0.1, TimerMode::Repeating),
            current_frame: 0,
        },
    ));
}

// ─── Transition de phase ───────────────────────────────────────────

fn update_player_phase(
    difficulty: Res<Difficulty>,
    textures: Res<ShipTextures>,
    mut query: Query<(&mut Handle<Image>, &mut ShipPhase), With<Player>>,
) {
    let boss_rotation_active = match difficulty.boss_music_start_time {
        Some(start) => difficulty.elapsed >= start + 3.0,
        None => false,
    };

    for (mut texture, mut ship) in query.iter_mut() {
        let target_phase = if boss_rotation_active {
            PlayerPhase::Phase3
        } else if difficulty.elapsed >= 10.0 {
            PlayerPhase::Phase2
        } else {
            PlayerPhase::Phase1
        };

        if ship.phase != target_phase {
            ship.phase = target_phase;
            ship.current_frame = 0;
            ship.timer.reset();

            match target_phase {
                PlayerPhase::Phase1 => {
                    ship.speed = PHASE_1_SPEED;
                    *texture = textures.phase_1.clone();
                }
                PlayerPhase::Phase2 => {
                    ship.speed = PHASE_2_SPEED;
                    *texture = textures.phase_2[0].clone();
                }
                PlayerPhase::Phase3 => {
                    ship.speed = PHASE_3_SPEED;
                    *texture = textures.phase_3[0].clone();
                }
            }
        }
    }
}

// ─── Mouvement ─────────────────────────────────────────────────────

const PLAYER_MARGIN: f32 = 64.0;

fn movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &ShipPhase), With<Player>>,
    difficulty: Res<Difficulty>,
    windows: Query<&Window>,
) {
    if difficulty.elapsed < 1.0 {
        return;
    }

    let window = windows.single();
    let half_w = window.width() / 2.0 - PLAYER_MARGIN;
    let half_h = window.height() / 2.0 - PLAYER_MARGIN;

    let (mut transform, ship) = query.single_mut();
    let mut direction = Vec3::ZERO;

    if keyboard.pressed(KeyCode::KeyW) {
        direction.y += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        direction.y -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        direction.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        direction.x += 1.0;
    }

    transform.translation += direction.normalize_or_zero() * ship.speed * 0.016;

    transform.translation.x = transform.translation.x.clamp(-half_w, half_w);
    transform.translation.y = transform.translation.y.clamp(-half_h, half_h);
}

// ─── Animation ─────────────────────────────────────────────────────

fn animate_ship(
    time: Res<Time>,
    textures: Res<ShipTextures>,
    mut query: Query<(&mut Handle<Image>, &mut ShipPhase), With<Player>>,
) {
    for (mut texture, mut ship) in query.iter_mut() {
        if ship.phase == PlayerPhase::Phase1 {
            continue;
        }

        ship.timer.tick(time.delta());
        if ship.timer.just_finished() {
            let frames = match ship.phase {
                PlayerPhase::Phase1 => continue,
                PlayerPhase::Phase2 => &textures.phase_2,
                PlayerPhase::Phase3 => &textures.phase_3,
            };
            ship.current_frame = (ship.current_frame + 1) % frames.len();
            *texture = frames[ship.current_frame].clone();
        }
    }
}

// ─── Rotation vers le réticule ─────────────────────────────────────

fn rotate_towards_crosshair(
    crosshair_q: Query<&Transform, (With<Crosshair>, Without<Player>)>,
    mut player_q: Query<&mut Transform, (With<Player>, Without<Crosshair>)>,
) {
    let crosshair_pos = crosshair_q.single().translation;
    let mut player_transform = player_q.single_mut();

    let direction = crosshair_pos - player_transform.translation;
    let angle = direction.y.atan2(direction.x) - std::f32::consts::FRAC_PI_2;
    player_transform.rotation = Quat::from_rotation_z(angle);
}

// ─── Flash blanc au boom ────────────────────────────────────────────

fn boom_flash_trigger(
    mut commands: Commands,
    mut boom_events: EventReader<BoomEvent>,
    player_q: Query<Entity, With<Player>>,
) {
    if boom_events.read().next().is_none() {
        return;
    }
    boom_events.read().for_each(drop);

    if let Ok(entity) = player_q.get_single() {
        commands
            .entity(entity)
            .insert(BoomFlash(Timer::from_seconds(
                BOOM_FLASH_DURATION,
                TimerMode::Once,
            )));
    }
}

fn boom_flash_update(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Sprite, &mut BoomFlash), With<Player>>,
) {
    for (entity, mut sprite, mut flash) in query.iter_mut() {
        flash.0.tick(time.delta());
        let t = flash.0.fraction();

        if flash.0.finished() {
            sprite.color = Color::WHITE;
            commands.entity(entity).remove::<BoomFlash>();
        } else {
            let intensity = 1.0 + (1.0 - t) * 8.0;
            sprite.color = Color::rgba(intensity, intensity, intensity, 1.0);
        }
    }
}
