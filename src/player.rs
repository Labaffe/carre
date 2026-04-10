//! Joueur : spawn, mouvement ZQSD, rotation vers le réticule, animation par phase.
//!
//! Phase 1 (0–10s)  : image statique ship_0.png, vitesse 200, Standard Missile.
//! Phase 2 (10s+)   : animation phase_2/ (20 frames), vitesse 400, Red Projectile.
//! Phase 3 (boss rotation) : animation phase_3/ (9 frames), vitesse 800, Blue Projectiles.

use crate::crosshair::Crosshair;
use crate::difficulty::{BoomEvent, Difficulty};
use crate::explosion::load_frames_from_folder;
use crate::pause::not_paused;
use crate::state::GameState;
use crate::weapon::Weapon;
use bevy::prelude::*;

// ─── Système de vies ──────────────────────────────────────────────

/// Nombre de vies au départ.
const PLAYER_MAX_LIVES: i32 = 3;
/// Durée d'invincibilité après un hit (secondes).
pub const INVINCIBLE_DURATION: f32 = 2.0;
/// Fréquence de clignotement pendant l'invincibilité (Hz).
const INVINCIBLE_BLINK_RATE: f32 = 3.0;
/// Fréquence de clignotement quand il reste 1 vie (Hz).
const LAST_LIFE_BLINK_RATE: f32 = 3.0;
/// Bonus de vitesse quand il reste 1 vie (multiplicateur).
const LAST_LIFE_SPEED_MULT: f32 = 1.25;

/// Nombre de vies restantes.
#[derive(Resource)]
pub struct PlayerLives {
    pub lives: i32,
}

impl Default for PlayerLives {
    fn default() -> Self {
        Self {
            lives: PLAYER_MAX_LIVES,
        }
    }
}

/// Invincibilité temporaire après un hit.
#[derive(Component)]
pub struct Invincible(pub Timer);

/// Marqueur pour les icônes de vie dans l'UI.
#[derive(Component)]
pub struct LivesUI;

/// Marqueur individuel pour chaque icône de vie.
#[derive(Component)]
struct LifeIcon(i32);

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerLives>()
            .add_systems(Startup, preload_ship_textures)
            .add_systems(OnEnter(GameState::Playing), (setup_player, setup_lives_ui))
            .add_systems(OnExit(GameState::Playing), cleanup_lives_ui)
            .add_systems(
                Update,
                (
                    movement,
                    rotate_towards_crosshair,
                    update_player_phase,
                    animate_ship,
                    boom_flash_trigger,
                    boom_flash_update,
                    update_invincibility,
                    last_life_blink,
                    update_lives_ui,
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

// ─── Invincibilité ────────────────────────────────────────────────

fn update_invincibility(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Sprite, &mut Invincible), With<Player>>,
) {
    for (entity, mut sprite, mut inv) in query.iter_mut() {
        inv.0.tick(time.delta());

        if inv.0.finished() {
            sprite.color = Color::WHITE;
            commands.entity(entity).remove::<Invincible>();
        } else {
            // Clignotement rapide : alternance visible/semi-transparent
            let blink =
                (inv.0.elapsed_secs() * INVINCIBLE_BLINK_RATE * std::f32::consts::TAU).sin();
            let alpha = if blink > 0.0 { 1.0 } else { 0.0 };
            sprite.color = Color::rgba(1.0, 1.0, 1.0, alpha);
        }
    }
}

// ─── Dernière vie : clignotement continu + boost vitesse ──────────

fn last_life_blink(
    lives: Res<PlayerLives>,
    difficulty: Res<Difficulty>,
    mut query: Query<(&mut Sprite, &mut ShipPhase, Option<&Invincible>), With<Player>>,
) {
    for (mut sprite, mut ship, invincible) in query.iter_mut() {
        // Boost de vitesse à 1 vie
        let base_speed = match ship.phase {
            PlayerPhase::Phase1 => PHASE_1_SPEED,
            PlayerPhase::Phase2 => PHASE_2_SPEED,
            PlayerPhase::Phase3 => PHASE_3_SPEED,
        };
        if lives.lives == 1 {
            ship.speed = base_speed * LAST_LIFE_SPEED_MULT;
        } else {
            ship.speed = base_speed;
        }

        // Clignotement dernière vie (style boss touché) — skip si déjà en invincibilité
        if lives.lives == 1 && invincible.is_none() {
            let t = (difficulty.elapsed * LAST_LIFE_BLINK_RATE * std::f32::consts::TAU).sin();
            let v = 1.0 + (t * 0.5 + 0.5) * 2.0; // pulse entre 1.0 et 3.0
            sprite.color = Color::rgba(v, v, v, 1.0);
        }
    }
}

// ─── UI des vies ──────────────────────────────────────────────────

fn setup_lives_ui(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut lives: ResMut<PlayerLives>,
) {
    *lives = PlayerLives::default();

    let texture = asset_server.load("images/player_ship/ship_0.png");

    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    top: Val::Px(20.0),
                    left: Val::Px(20.0),
                    column_gap: Val::Px(12.0),
                    ..default()
                },
                ..default()
            },
            LivesUI,
        ))
        .with_children(|parent| {
            for i in 0..PLAYER_MAX_LIVES {
                parent.spawn((
                    ImageBundle {
                        image: UiImage::new(texture.clone()),
                        style: Style {
                            width: Val::Px(64.0),
                            height: Val::Px(64.0),
                            ..default()
                        },
                        ..default()
                    },
                    LifeIcon(i),
                ));
            }
        });
}

fn update_lives_ui(lives: Res<PlayerLives>, mut icons: Query<(&LifeIcon, &mut Visibility)>) {
    for (icon, mut vis) in icons.iter_mut() {
        if icon.0 < lives.lives {
            *vis = Visibility::Visible;
        } else {
            *vis = Visibility::Hidden;
        }
    }
}

fn cleanup_lives_ui(mut commands: Commands, query: Query<Entity, With<LivesUI>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}
