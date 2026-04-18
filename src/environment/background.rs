//! Background spatial scrollant en boucle.
//!
//! **Phase normale** : 2 tiles verticales, scroll vers le bas.
//! **Phase boss** (3 s après boss.ogg) : grille 3×3 qui scroll ET tourne
//! en même temps que la planète, simulant une orbite.

use crate::game_manager::difficulty::Difficulty;
use crate::game_manager::state::GameState;
use crate::level::level::{LevelConfig, LevelSetupSet};
use crate::level::levels::ScrollDirection;
use bevy::prelude::*;

pub struct BackgroundPlugin;

impl Plugin for BackgroundPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(GameState::Playing),
            (setup_background.after(LevelSetupSet), spawn_planet),
        )
        .add_systems(
            Update,
            (scroll_background, animate_planet).run_if(in_state(GameState::Playing)),
        );
    }
}

#[derive(Component)]
pub struct Background;

#[derive(Component)]
pub struct Planet;

// ─── Constantes background ─────────────────────────────────────────

/// Largeur d'une tile de background (px).
const BG_TILE_WIDTH: f32 = 5796.0;
/// Hauteur d'une tile de background (px).
const BG_TILE_HEIGHT: f32 = 1534.0;
/// Vitesse du scroll du background pendant le boss (px/s).
const BOSS_BG_SCROLL_SPEED: f32 = 150.0;
/// Vitesse de rotation du background pendant le boss (rad/s, = planète).
const BOSS_BG_ROTATION_SPEED: f32 = 0.50;

fn setup_background(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    existing: Query<Entity, With<Background>>,
    config: Res<LevelConfig>,
) {
    if !existing.is_empty() {
        return;
    }

    let bg = asset_server.load(config.background_tile);
    let tile_rot = Quat::from_rotation_z(config.scroll_direction.bg_tile_rotation());

    // Après rotation 90°, la tile fait BG_TILE_HEIGHT de large (1534px).
    // Pour couvrir l'écran (~1920px) en horizontal, il faut 3 tiles centrées.
    let tile_range: Vec<i32> = if config.scroll_direction.is_horizontal() {
        vec![-1, 0, 1] // 3 tiles centrées autour de 0
    } else {
        vec![0, 1] // 2 tiles verticales
    };

    for i in tile_range {
        let pos = if config.scroll_direction.is_horizontal() {
            Vec3::new(BG_TILE_HEIGHT * i as f32, 0.0, -1.0)
        } else {
            Vec3::new(0.0, BG_TILE_HEIGHT * i as f32, -1.0)
        };

        commands.spawn((
            SpriteBundle {
                texture: bg.clone(),
                transform: Transform {
                    translation: pos,
                    rotation: tile_rot,
                    ..default()
                },
                ..default()
            },
            Background,
        ));
    }
}

/// Fait défiler le background.
/// - Avant le boss : 2 tiles, scroll dans la direction du niveau.
/// - Pendant le boss (niveau 1) : grille qui scroll et tourne (orbite planétaire).
fn scroll_background(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut query: Query<(Entity, &mut Transform), With<Background>>,
    windows: Query<&Window>,
    time: Res<Time>,
    mut difficulty: ResMut<Difficulty>,
    config: Res<LevelConfig>,
) {
    let boss_bg_active = match difficulty.boss_music_start_time {
        Some(start) => difficulty.elapsed - start >= 3.0,
        None => false,
    };

    // Nombre total de tiles en mode boss (2 existantes + 4 ajoutées).
    const BOSS_TILE_COUNT: f32 = 6.0;

    if boss_bg_active {
        // ── Mode boss : scroll + rotation (uniquement pour les niveaux verticaux) ──
        if !difficulty.boss_bg_initialized {
            difficulty.boss_bg_initialized = true;

            let bg = asset_server.load(config.background_tile);
            for row in [-2_i32, -1, 2, 3] {
                commands.spawn((
                    SpriteBundle {
                        texture: bg.clone(),
                        transform: Transform::from_xyz(0.0, row as f32 * BG_TILE_HEIGHT, -1.0),
                        ..default()
                    },
                    Background,
                ));
            }
            return;
        }

        let boss_bg_elapsed = difficulty.elapsed - difficulty.boss_music_start_time.unwrap() - 3.0;
        let angle = boss_bg_elapsed * BOSS_BG_ROTATION_SPEED;
        let rotation = Quat::from_rotation_z(angle);
        let scroll_total = boss_bg_elapsed * BOSS_BG_SCROLL_SPEED;

        let grid_h = BOSS_TILE_COUNT * BG_TILE_HEIGHT;
        let half_grid_h = grid_h / 2.0;

        let window = windows.single();
        let half_h = window.height() / 2.0;
        let planet_x = (difficulty.elapsed * 0.3).sin() * 15.0;
        let planet_y = -(half_h + 700.0) + (difficulty.elapsed * 0.2).cos() * 10.0;
        let pivot = Vec3::new(planet_x, planet_y, 0.0);

        let mut tiles: Vec<(Entity, Mut<Transform>)> = query.iter_mut().collect();
        let count = tiles.len() as f32;
        let half_count = count / 2.0;
        for (idx, (_, tf)) in tiles.iter_mut().enumerate() {
            let row = idx as f32 - half_count + 0.5;

            let raw_y = row * BG_TILE_HEIGHT - scroll_total;
            let wrapped_y = ((raw_y + half_grid_h).rem_euclid(grid_h)) - half_grid_h;

            let local_pos = Vec3::new(0.0, wrapped_y, 0.0);
            let rotated = rotation.mul_vec3(local_pos);
            tf.translation = Vec3::new(pivot.x + rotated.x, pivot.y + rotated.y, -1.0);
            tf.rotation = rotation;
        }
    } else {
        // ── Scroll normal dans la direction du niveau ──
        let base_speed = 150.0;
        let speed = if let Some(override_speed) = difficulty.bg_speed_override {
            override_speed
        } else {
            base_speed * (1.0 + difficulty.factor * 3.0)
        };

        let delta = speed * time.delta_seconds();

        for (_, mut transform) in query.iter_mut() {
            match config.scroll_direction {
                ScrollDirection::Down => {
                    transform.translation.y -= delta;
                    if transform.translation.y <= -BG_TILE_HEIGHT {
                        transform.translation.y += BG_TILE_HEIGHT * 2.0;
                    }
                }
                ScrollDirection::Up => {
                    transform.translation.y += delta;
                    if transform.translation.y >= BG_TILE_HEIGHT {
                        transform.translation.y -= BG_TILE_HEIGHT * 2.0;
                    }
                }
                ScrollDirection::Left => {
                    transform.translation.x -= delta;
                    // 3 tiles centrées (-S, 0, S) : recycler à -1.5×S pour éviter les trous
                    if transform.translation.x <= -BG_TILE_HEIGHT * 1.5 {
                        transform.translation.x += BG_TILE_HEIGHT * 3.0;
                    }
                }
                ScrollDirection::Right => {
                    transform.translation.x += delta;
                    if transform.translation.x >= BG_TILE_HEIGHT * 1.5 {
                        transform.translation.x -= BG_TILE_HEIGHT * 3.0;
                    }
                }
            }
        }
    }
}

// ─── Planète ────────────────────────────────────────────────────────

/// Durée de l'animation de zoom (secondes).
const PLANET_ANIM_DURATION: f32 = 10.0;
/// Vitesse de rotation de la planète pendant le boss (après 3s de musique boss).
const PLANETE_BOSS_ROTATION_SPEED: f32 = 0.50;

fn spawn_planet(mut commands: Commands, asset_server: Res<AssetServer>, windows: Query<&Window>) {
    let window = windows.single();
    let half_h = window.height() / 2.0;

    commands.spawn((
        SpriteBundle {
            texture: asset_server.load("images/backgrounds/planete.png"),
            sprite: Sprite {
                color: Color::WHITE,
                ..default()
            },
            transform: Transform {
                translation: Vec3::new(0.0, -(half_h + 900.0), -0.5),
                scale: Vec3::splat(1.0),
                ..default()
            },
            ..default()
        },
        Planet,
    ));
}

fn animate_planet(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut difficulty: ResMut<Difficulty>,
    windows: Query<&Window>,
    mut planet_q: Query<&mut Transform, With<Planet>>,
) {
    // Apparition contrôlée par le système de niveau
    let planet_appear_time = match difficulty.planet_appear_elapsed {
        Some(t) => t,
        None => return,
    };

    // Son landing 6.3s avant la fin de l'animation
    let landing_time = planet_appear_time + PLANET_ANIM_DURATION - 6.3;
    if difficulty.elapsed >= landing_time && !difficulty.landing_played {
        difficulty.landing_played = true;
        commands.spawn(AudioBundle {
            source: asset_server.load("audio/sfx/landing.ogg"),
            settings: PlaybackSettings::DESPAWN,
        });
    }

    if difficulty.elapsed < planet_appear_time {
        return;
    }

    let window = windows.single();
    let half_h = window.height() / 2.0;

    let progress =
        ((difficulty.elapsed - planet_appear_time) / PLANET_ANIM_DURATION).clamp(0.0, 1.0);

    for mut transform in planet_q.iter_mut() {
        // Courbe ease-in-out : doux au début et à la fin
        let eased = progress * progress * (3.0 - 2.0 * progress);

        // Scale : 1.0→9.0 (zoom bien plus prononcé)
        let scale = 1.0 + eased * 4.0;
        transform.scale = Vec3::splat(scale);

        // Position Y : remonte davantage pour montrer plus de surface
        let start_y = -(half_h + 900.0);
        let end_y = -(half_h + 600.0);
        transform.translation.y = start_y + (end_y - start_y) * eased;

        // Position X : centre avec léger mouvement d'orbite
        let orbit_x = (difficulty.elapsed * 0.3).sin() * 15.0;
        let orbit_y = (difficulty.elapsed * 0.2).cos() * 10.0;
        transform.translation.x = orbit_x;
        transform.translation.y += orbit_y;

        // Rotation : accélère 3s après le lancement de la musique boss.
        // On accumule l'angle pour éviter un saut brutal au changement de vitesse.
        let base_speed = 0.02;
        let angle = match difficulty.boss_music_start_time {
            Some(start) if difficulty.elapsed - start >= 3.0 => {
                let switch_time = start + 3.0;
                // Angle accumulé avant la transition + angle depuis la transition
                let angle_before = switch_time * base_speed;
                let elapsed_since = difficulty.elapsed - switch_time;
                angle_before + elapsed_since * PLANETE_BOSS_ROTATION_SPEED
            }
            _ => difficulty.elapsed * base_speed,
        };
        transform.rotation = Quat::from_rotation_z(angle);
    }
}
