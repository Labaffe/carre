use bevy::prelude::*;
use crate::crosshair::Crosshair;
use crate::difficulty::Difficulty;
use crate::state::GameState;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_player)
            .add_systems(
                Update,
                (movement, rotate_towards_crosshair)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

#[derive(Component)]
pub struct Player;

fn setup_player(mut commands: Commands, asset_server: Res<AssetServer>) {
    spawn_player(&mut commands, &asset_server);
}

pub fn spawn_player(commands: &mut Commands, asset_server: &Res<AssetServer>) {
    commands.spawn((
        SpriteBundle {
            texture: asset_server.load("images/vaisseau.png"),
            sprite: Sprite {
                custom_size: Some(Vec2::new(64.0, 64.0)),
                ..default()
            },
            ..default()
        },
        Player,
    ));
}

fn movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<Player>>,
    difficulty: Res<Difficulty>,
) {
    let mut transform = query.single_mut();
    let speed = if difficulty.elapsed >= 10.0 { 400.0 } else { 200.0 };
    let mut direction = Vec3::ZERO;

    if keyboard.pressed(KeyCode::KeyW) { direction.y += 1.0; }
    if keyboard.pressed(KeyCode::KeyS) { direction.y -= 1.0; }
    if keyboard.pressed(KeyCode::KeyA) { direction.x -= 1.0; }
    if keyboard.pressed(KeyCode::KeyD) { direction.x += 1.0; }

    transform.translation += direction.normalize_or_zero() * speed * 0.016;
}

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
