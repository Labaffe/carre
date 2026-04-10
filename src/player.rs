//! Joueur : spawn, mouvement ZQSD, rotation vers le réticule, animation du vaisseau.
//!
//! À 10 secondes, la vitesse de déplacement double (200 → 400 px/s)
//! et le sprite commence à cycler ses 9 frames d'animation.

use bevy::prelude::*;
use crate::crosshair::Crosshair;
use crate::difficulty::{BoomEvent, Difficulty};
use crate::pause::PauseState;
use crate::state::GameState;
use crate::weapon::Weapon;

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
                (movement, rotate_towards_crosshair, animate_ship, boom_flash_trigger, boom_flash_update)
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

#[derive(Component)]
struct ShipAnimation {
    timer: Timer,
    current_frame: usize,
    active: bool,
}

#[derive(Resource)]
struct ShipTextures(Vec<Handle<Image>>);

fn preload_ship_textures(mut commands: Commands, asset_server: Res<AssetServer>) {
    let textures = (0..=8)
        .map(|i| asset_server.load(format!("images/player_ship/ship_{}.png", i)))
        .collect();
    commands.insert_resource(ShipTextures(textures));
}

fn setup_player(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    windows: Query<&Window>,
) {
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
        ShipAnimation {
            timer: Timer::from_seconds(0.1, TimerMode::Repeating),
            current_frame: 0,
            active: false,
        },
    ));
}

/// Déplacement WASD (ZQSD en AZERTY). Vitesse doublée après 10 secondes.
/// Marge en pixels par rapport au bord de l'écran (demi-taille du sprite joueur).
const PLAYER_MARGIN: f32 = 64.0;

fn movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<Player>>,
    difficulty: Res<Difficulty>,
    windows: Query<&Window>,
) {
    // Bloquer le mouvement pendant la première seconde
    if difficulty.elapsed < 1.0 {
        return;
    }

    let window = windows.single();
    let half_w = window.width() / 2.0 - PLAYER_MARGIN;
    let half_h = window.height() / 2.0 - PLAYER_MARGIN;

    let mut transform = query.single_mut();
    let speed = if difficulty.elapsed >= 10.0 { 400.0 } else { 200.0 };
    let mut direction = Vec3::ZERO;

    if keyboard.pressed(KeyCode::KeyW) { direction.y += 1.0; }
    if keyboard.pressed(KeyCode::KeyS) { direction.y -= 1.0; }
    if keyboard.pressed(KeyCode::KeyA) { direction.x -= 1.0; }
    if keyboard.pressed(KeyCode::KeyD) { direction.x += 1.0; }

    transform.translation += direction.normalize_or_zero() * speed * 0.016;

    transform.translation.x = transform.translation.x.clamp(-half_w, half_w);
    transform.translation.y = transform.translation.y.clamp(-half_h, half_h);
}

/// Animation du vaisseau : cycle les 9 frames (ship_0 à ship_8) à partir de 10 secondes.
fn animate_ship(
    time: Res<Time>,
    difficulty: Res<Difficulty>,
    textures: Res<ShipTextures>,
    mut query: Query<(&mut Handle<Image>, &mut ShipAnimation), With<Player>>,
) {
    for (mut texture, mut anim) in query.iter_mut() {
        if !anim.active {
            if difficulty.elapsed >= 10.0 {
                anim.active = true;
            } else {
                continue;
            }
        }

        anim.timer.tick(time.delta());
        if anim.timer.just_finished() {
            anim.current_frame = (anim.current_frame + 1) % textures.0.len();
            *texture = textures.0[anim.current_frame].clone();
        }
    }
}

/// Fait pivoter le vaisseau pour pointer vers le réticule.
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

/// Déclenche un flash blanc sur le vaisseau à chaque BoomEvent.
fn boom_flash_trigger(
    mut commands: Commands,
    mut boom_events: EventReader<BoomEvent>,
    player_q: Query<Entity, With<Player>>,
) {
    if boom_events.read().next().is_none() {
        return;
    }
    // Consommer tous les événements restants
    boom_events.read().for_each(drop);

    if let Ok(entity) = player_q.get_single() {
        commands
            .entity(entity)
            .insert(BoomFlash(Timer::from_seconds(BOOM_FLASH_DURATION, TimerMode::Once)));
    }
}

/// Anime le flash blanc : blanc intense → couleur normale.
fn boom_flash_update(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Sprite, &mut BoomFlash), With<Player>>,
) {
    for (entity, mut sprite, mut flash) in query.iter_mut() {
        flash.0.tick(time.delta());
        let t = flash.0.fraction(); // 0 → 1

        if flash.0.finished() {
            sprite.color = Color::WHITE;
            commands.entity(entity).remove::<BoomFlash>();
        } else {
            // Flash blanc intense qui s'estompe : surbrillance au début, retour à la normale
            let intensity = 1.0 + (1.0 - t) * 8.0; // 9.0 → 1.0
            sprite.color = Color::rgba(intensity, intensity, intensity, 1.0);
        }
    }
}
