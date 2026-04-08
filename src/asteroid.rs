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
    spawner
        .timer
        .set_duration(Duration::from_secs_f32(difficulty.spawn_interval()));
    spawner.timer.tick(time.delta());

    if !spawner.timer.just_finished() {
        return;
    }

    let window = windows.single();
    let x = fastrand::f32() * window.width() - window.width() / 2.0;

    let is_small = fastrand::bool();
    let texture = textures.0[fastrand::usize(..textures.0.len())].clone();

    let transform = Transform::from_xyz(x, 500.0, 0.0).with_rotation(Quat::from_rotation_z(
        fastrand::f32() * std::f32::consts::TAU,
    ));

    // 1. TAILLE générée en premier
    let side = if is_small {
        fastrand::f32() * 40.0 + 35.0 // 35 → 75 px
    } else {
        fastrand::f32() * 60.0 + 120.0 // 120 → 180 px
    };
    let size = Vec2::splat(side);
    let radius = side * 0.45;

    // 2. PV dérivés de la taille
    //    < 35px → toujours 1 PV, sinon interpolation linéaire 1→5 jusqu'à 180px, max 5
    let health = if side < 35.0 {
        1
    } else {
        ((side - 35.0) / (180.0 - 35.0) * 4.0 + 1.0)
            .round()
            .clamp(1.0, 5.0) as i32
    };

    // 3. VITESSE dérivée de la taille (inversement proportionnelle)
    //    35px → ~300 px/s   |   180px → ~50 px/s
    let speed = 300.0 - (side - 35.0) / (180.0 - 35.0) * 250.0;
    let velocity = Vec3::new(0.0, -speed, 0.0) * difficulty.factor;

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
            velocity,
            radius,
            health,
            size,
        },
    ));
}

fn move_asteroids(mut query: Query<(&mut Transform, &Asteroid)>, time: Res<Time>) {
    for (mut transform, asteroid) in query.iter_mut() {
        transform.translation += asteroid.velocity * time.delta_seconds();
    }
}
