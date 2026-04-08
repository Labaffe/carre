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
                (spawn_asteroids, move_asteroids).run_if(in_state(GameState::Playing)),
            );
    }
}

#[derive(Component)]
pub struct Asteroid {
    pub velocity: Vec3,
    pub radius: f32,
    pub health: i32,
    pub size: Vec2,
}

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
    spawner.timer.set_duration(Duration::from_secs_f32(difficulty.spawn_interval()));
    spawner.timer.tick(time.delta());

    if !spawner.timer.just_finished() {
        return;
    }

    let window = windows.single();
    let x = fastrand::f32() * window.width() - window.width() / 2.0;

    let is_small = fastrand::bool();
    let texture = textures.0[fastrand::usize(..textures.0.len())].clone();

    let transform = Transform::from_xyz(x, 500.0, 0.0)
        .with_rotation(Quat::from_rotation_z(fastrand::f32() * std::f32::consts::TAU));

    let (size, radius, health, base_velocity) = if is_small {
        (
            Vec2::new(48.0, 48.0),
            20.0,
            1,
            Vec3::new(0.0, -120.0 * (fastrand::f32() + 1.0), 0.0),
        )
    } else {
        (
            Vec2::new(102.5, 102.5),
            47.0,
            5,
            Vec3::new(0.0, -100.0 * (fastrand::f32() + 1.0), 0.0),
        )
    };

    let velocity = base_velocity * difficulty.factor;

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
        Asteroid { velocity, radius, health, size },
    ));
}

fn move_asteroids(mut query: Query<(&mut Transform, &Asteroid)>, time: Res<Time>) {
    for (mut transform, asteroid) in query.iter_mut() {
        transform.translation += asteroid.velocity * time.delta_seconds();
    }
}
