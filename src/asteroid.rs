use bevy::prelude::*;

pub struct AsteroidPlugin;

impl Plugin for AsteroidPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AsteroidSpawner::default())
            .add_systems(Update, (spawn_asteroids, move_asteroids));
    }
}

#[derive(Component)]
struct Asteroid {
    velocity: Vec3,
}

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

fn spawn_asteroids(
    windows: Query<&Window>,
    mut commands: Commands,
    mut spawner: ResMut<AsteroidSpawner>,
    time: Res<Time>,
    asset_server: Res<AssetServer>,
) {
    spawner.timer.tick(time.delta());

    if spawner.timer.just_finished() {
        let window = windows.single();
        let x = fastrand::f32() * window.width() - window.width() / 2.0;
        let y = 500.0;

        let is_small = fastrand::bool();

        let transform = Transform::from_xyz(x, y, 0.0).with_rotation(Quat::from_rotation_z(
            fastrand::f32() * std::f32::consts::TAU,
        ));

        let (image, size, velocity) = if is_small {
            (
                "asteroid_1.png",
                Vec2::new(24.0 * 2.0, 24.0 * 2.0),
                Vec3::new(0.0, -200.0 * (fastrand::f32() + 1.0), 0.0),
            )
        } else {
            (
                "asteroid_2.png",
                Vec2::new(41.0 * 2.5, 41.0 * 2.5),
                Vec3::new(0.0, -100.0 * (fastrand::f32() + 1.0), 0.0),
            )
        };

        commands.spawn((
            SpriteBundle {
                texture: asset_server.load(image),
                sprite: Sprite {
                    custom_size: Some(size),
                    ..default()
                },
                transform,
                ..default()
            },
            Asteroid { velocity },
        ));
    }
}

fn move_asteroids(mut query: Query<(&mut Transform, &Asteroid)>, time: Res<Time>) {
    for (mut transform, asteroid) in query.iter_mut() {
        transform.translation += asteroid.velocity * time.delta_seconds();
    }
}
