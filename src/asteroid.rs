use crate::difficulty::Difficulty;
use crate::state::GameState;
use bevy::prelude::*;
use std::time::Duration;

pub struct AsteroidPlugin;

impl Plugin for AsteroidPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AsteroidSpawner::default())
            .add_systems(Startup, preload_asteroid_textures)
            .add_systems(
                Update,
                (spawn_asteroids, move_asteroids, animate_hit_flash)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

#[derive(Component)]
pub struct Asteroid {
    pub base_velocity: Vec3,
    pub radius: f32,
    pub health: i32,
    pub size: Vec2,
}

/// Inséré sur l'astéroïde au moment d'un hit. Le timer contrôle la durée du flash.
#[derive(Component)]
pub struct HitFlash(pub Timer);

#[derive(Resource)]
struct AsteroidTextures(Vec<Handle<Image>>);

#[derive(Resource)]
struct AsteroidSpawner {
    timer: Timer,
}

impl Default for AsteroidSpawner {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(1.0, TimerMode::Repeating),
        }
    }
}

fn preload_asteroid_textures(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handles = (0..=16)
        .map(|i| asset_server.load(format!("images/asteroids/asteroid_x{:03}.png", i)))
        .collect();
    commands.insert_resource(AsteroidTextures(handles));
}

fn spawn_asteroids(
    windows: Query<&Window>,
    mut commands: Commands,
    mut spawner: ResMut<AsteroidSpawner>,
    time: Res<Time>,
    difficulty: Res<Difficulty>,
    textures: Res<AsteroidTextures>,
) {
    spawner
        .timer
        .set_duration(Duration::from_secs_f32(difficulty.spawn_interval()));
    spawner.timer.tick(time.delta());

    if !spawner.timer.just_finished() {
        return;
    }

    let window = windows.single();
    let x = fastrand::f32() * window.width() - window.width() / 2.0;
    let is_small = fastrand::f32() < 0.5; // 30% de petits
    let texture = textures.0[fastrand::usize(..textures.0.len())].clone();

    let transform = Transform::from_xyz(x, 500.0, 0.0).with_rotation(Quat::from_rotation_z(
        fastrand::f32() * std::f32::consts::TAU,
    ));

    let side = if is_small {
        fastrand::f32() * 30.0 + 60.0 // 60-90px
    } else {
        fastrand::f32() * 60.0 + 120.0 // 120-180px
    };
    let size = Vec2::splat(side);
    let radius = side * 0.30;

    let health = if side < 35.0 {
        1
    } else {
        ((side - 35.0) / (180.0 - 35.0) * 4.0 + 1.0)
            .round()
            .clamp(1.0, 5.0) as i32
    };

    let speed = 150.0 - (side - 35.0) / (180.0 - 35.0) * 100.0;
    let base_velocity = Vec3::new(0.0, -speed, 0.0);

    commands.spawn((
        SpriteBundle {
            texture,
            sprite: Sprite {
                custom_size: Some(size),
                ..default()
            },
            transform,
            ..default()
        },
        Asteroid {
            base_velocity,
            radius,
            health,
            size,
        },
    ));
}

/// Flash au hit : alterne rapidement entre sprite visible et invisible.
/// Note : dans Bevy, sprite.color est un multiplicateur de texture.
/// Color::WHITE = apparence normale. Pour un flash visible, on joue sur l'alpha.
fn animate_hit_flash(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Sprite, &mut HitFlash), With<Asteroid>>,
) {
    for (entity, mut sprite, mut flash) in query.iter_mut() {
        flash.0.tick(time.delta());

        if flash.0.finished() {
            sprite.color = Color::WHITE;
            commands.entity(entity).remove::<HitFlash>();
        } else {
            // alterne entre blanc opaque et quasi-invisible à 35 Hz
            let blink = (flash.0.elapsed_secs() * 35.0).sin();
            sprite.color = if blink > 0.0 {
                Color::WHITE // sprite normal
            } else {
                Color::rgba(1.0, 1.0, 1.0, 0.0) // invisible → effet flash blanc
            };
        }
    }
}

fn move_asteroids(
    mut query: Query<(&mut Transform, &Asteroid)>,
    time: Res<Time>,
    difficulty: Res<Difficulty>,
) {
    for (mut transform, asteroid) in query.iter_mut() {
        transform.translation += asteroid.base_velocity * difficulty.factor * time.delta_seconds();
    }
}
