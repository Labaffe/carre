//! Background spatial scrollant en boucle.
//! Deux copies de l'image se suivent verticalement pour un défilement infini.
//! La vitesse de scroll est proportionnelle au carré du facteur de difficulté.
//! Spawné à l'entrée du Playing, caché au game over, réaffiché au restart.

use crate::difficulty::Difficulty;
use crate::state::GameState;
use bevy::prelude::*;

pub struct BackgroundPlugin;

impl Plugin for BackgroundPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), (setup_background, spawn_planet))
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

fn setup_background(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    existing: Query<Entity, With<Background>>,
) {
    // Ne pas re-spawner si le background existe déjà (cas du restart après game over)
    if !existing.is_empty() {
        // Réafficher les backgrounds cachés
        return;
    }

    let bg = asset_server.load("images/space_background.png");

    for i in 0..2 {
        commands.spawn((
            SpriteBundle {
                texture: bg.clone(),
                transform: Transform::from_xyz(0.0, 1536.0 * i as f32, -1.0),
                ..default()
            },
            Background,
        ));
    }
}

/// Fait défiler les deux sprites de background vers le bas.
/// Quand un sprite sort de l'écran, il est repositionné au-dessus de l'autre.
fn scroll_background(
    mut query: Query<&mut Transform, With<Background>>,
    time: Res<Time>,
    difficulty: Res<Difficulty>,
) {
    let base_speed = 150.0;
    let image_height = 1536.0;
    // Après 26.7s : vitesse décroissante indépendante de la difficulté.
    // Sinon : scaling basé sur le facteur.
    let speed = if let Some(override_speed) = difficulty.bg_speed_override {
        override_speed
    } else {
        base_speed * (1.0 + difficulty.factor * 3.0)
    };

    for mut transform in query.iter_mut() {
        transform.translation.y -= speed * time.delta_seconds();

        if transform.translation.y <= -image_height {
            transform.translation.y += image_height * 2.0;
        }
    }
}

// ─── Planète ────────────────────────────────────────────────────────

/// Temps d'apparition de la planète (secondes).
const PLANET_APPEAR_TIME: f32 = 31.0;
/// Durée de l'animation de zoom (secondes).
const PLANET_ANIM_DURATION: f32 = 4.0;

fn spawn_planet(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    windows: Query<&Window>,
) {
    let window = windows.single();
    let half_h = window.height() / 2.0;

    commands.spawn((
        SpriteBundle {
            texture: asset_server.load("images/planete.png"),
            sprite: Sprite {
                color: Color::rgba(1.0, 1.0, 1.0, 0.0),
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
    difficulty: Res<Difficulty>,
    windows: Query<&Window>,
    mut planet_q: Query<(&mut Transform, &mut Sprite), With<Planet>>,
) {
    if difficulty.elapsed < PLANET_APPEAR_TIME {
        return;
    }

    let window = windows.single();
    let half_h = window.height() / 2.0;

    let progress = ((difficulty.elapsed - PLANET_APPEAR_TIME) / PLANET_ANIM_DURATION).clamp(0.0, 1.0);

    for (mut transform, mut sprite) in planet_q.iter_mut() {
        // Alpha : apparition douce
        sprite.color.set_a(progress.clamp(0.0, 1.0));

        // Courbe ease-in-out : doux au début et à la fin
        let eased = progress * progress * (3.0 - 2.0 * progress);

        // Scale : 1.0→5.0 (bien zoomé)
        let scale = 1.0 + eased * 4.0;
        transform.scale = Vec3::splat(scale);

        // Position Y : presque entièrement sous l'écran, seul un petit arc dépasse
        let start_y = -(half_h + 900.0);
        let end_y = -(half_h + 700.0);
        transform.translation.y = start_y + (end_y - start_y) * eased;

        // Position X : centre avec léger mouvement d'orbite
        let orbit_x = (difficulty.elapsed * 0.3).sin() * 15.0;
        let orbit_y = (difficulty.elapsed * 0.2).cos() * 10.0;
        transform.translation.x = orbit_x;
        transform.translation.y += orbit_y;

        // Rotation très lente sur elle-même
        transform.rotation = Quat::from_rotation_z(difficulty.elapsed * 0.02);
    }
}
