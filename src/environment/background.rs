//! Background spatial scrollant en boucle.
//!
//! Scroll dans la direction du niveau (`LevelConfig.scroll_direction`).
//! Vitesse dérivée de `difficulty.factor` ou override via `bg_speed_override`
//! (utilisé par la décélération avant le boss).
//!
//! Note : l'ancien mode "arène boss" (grille 3×3 qui orbite autour de la
//! planète) a été retiré avec le système de phases temporelles.

use crate::audio::{Sfx, SfxPlayer};
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
            Sprite { image: bg.clone(), ..default() },
            Transform {
                translation: pos,
                rotation: tile_rot,
                ..default()
            },
            Background,
        ));
    }
}

/// Fait défiler le background dans la direction du niveau.
fn scroll_background(
    mut query: Query<&mut Transform, With<Background>>,
    time: Res<Time>,
    difficulty: Res<Difficulty>,
    config: Res<LevelConfig>,
) {
    let base_speed = 150.0;
    let speed = if let Some(override_speed) = difficulty.bg_speed_override {
        override_speed
    } else {
        base_speed * (1.0 + difficulty.factor * 3.0)
    };

    let delta = speed * time.delta_secs();

    for mut transform in query.iter_mut() {
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

// ─── Planète ────────────────────────────────────────────────────────

/// Durée de l'animation de zoom (secondes).
const PLANET_ANIM_DURATION: f32 = 10.0;
/// Vitesse de rotation lente constante de la planète (rad/s).
const PLANET_ROTATION_SPEED: f32 = 0.02;

fn spawn_planet(mut commands: Commands, asset_server: Res<AssetServer>, windows: Query<&Window>) {
    let window = windows.single().unwrap();
    let half_h = window.height() / 2.0;

    commands.spawn((
        Sprite {
            image: asset_server.load("images/backgrounds/planete.png"),
            color: Color::WHITE,
            ..default()
        },
        Transform {
            translation: Vec3::new(0.0, -(half_h + 900.0), -0.5),
            scale: Vec3::splat(1.0),
            ..default()
        },
        Planet,
    ));
}

fn animate_planet(
    mut difficulty: ResMut<Difficulty>,
    windows: Query<&Window>,
    mut planet_q: Query<&mut Transform, With<Planet>>,
    mut sfx: SfxPlayer,
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
        sfx.play(Sfx::PlanetLanding);
    }

    if difficulty.elapsed < planet_appear_time {
        return;
    }

    let window = windows.single().unwrap();
    let half_h = window.height() / 2.0;

    let progress =
        ((difficulty.elapsed - planet_appear_time) / PLANET_ANIM_DURATION).clamp(0.0, 1.0);

    for mut transform in planet_q.iter_mut() {
        // Courbe ease-in-out
        let eased = progress * progress * (3.0 - 2.0 * progress);

        // Scale : 1.0 → 5.0
        let scale = 1.0 + eased * 4.0;
        transform.scale = Vec3::splat(scale);

        // Position Y : remonte
        let start_y = -(half_h + 900.0);
        let end_y = -(half_h + 600.0);
        transform.translation.y = start_y + (end_y - start_y) * eased;

        // Léger mouvement d'orbite
        let orbit_x = (difficulty.elapsed * 0.3).sin() * 15.0;
        let orbit_y = (difficulty.elapsed * 0.2).cos() * 10.0;
        transform.translation.x = orbit_x;
        transform.translation.y += orbit_y;

        // Rotation lente constante
        transform.rotation = Quat::from_rotation_z(difficulty.elapsed * PLANET_ROTATION_SPEED);
    }
}
