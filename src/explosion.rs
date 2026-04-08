use bevy::prelude::*;

pub struct ExplosionPlugin;

impl Plugin for ExplosionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, animate_explosions);
    }
}

#[derive(Component)]
struct Explosion {
    frames: Vec<Handle<Image>>,
    current_frame: usize,
    timer: Timer,
}

pub fn spawn_explosion(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    position: Vec3,
    size: Vec2,
) {
    let frames = vec![
        asset_server.load("images/explosion_1.png"),
        asset_server.load("images/explosion_2.png"),
        asset_server.load("images/explosion_3.png"),
        asset_server.load("images/explosion_4.png"),
    ];

    commands.spawn((
        SpriteBundle {
            texture: frames[0].clone(),
            sprite: Sprite {
                custom_size: Some(size),
                ..default()
            },
            transform: Transform::from_translation(position),
            ..default()
        },
        Explosion {
            frames,
            current_frame: 0,
            timer: Timer::from_seconds(0.1, TimerMode::Repeating),
        },
    ));
}

fn animate_explosions(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Handle<Image>, &mut Explosion)>,
) {
    for (entity, mut texture, mut explosion) in query.iter_mut() {
        explosion.timer.tick(time.delta());

        if explosion.timer.just_finished() {
            explosion.current_frame += 1;
            if explosion.current_frame >= explosion.frames.len() {
                commands.entity(entity).despawn();
            } else {
                *texture = explosion.frames[explosion.current_frame].clone();
            }
        }
    }
}
