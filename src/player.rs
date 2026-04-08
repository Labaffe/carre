use bevy::prelude::*;
use crate::crosshair::Crosshair;
use crate::difficulty::Difficulty;
use crate::state::GameState;
use crate::weapon::Weapon;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_player, preload_ship_textures))
            .add_systems(
                Update,
                (movement, rotate_towards_crosshair, animate_ship)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

#[derive(Component)]
pub struct Player;

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

fn setup_player(mut commands: Commands, asset_server: Res<AssetServer>) {
    spawn_player(&mut commands, &asset_server);
}

pub fn spawn_player(commands: &mut Commands, asset_server: &Res<AssetServer>) {
    commands.spawn((
        SpriteBundle {
            texture: asset_server.load("images/player_ship/ship_0.png"),
            sprite: Sprite {
                custom_size: Some(Vec2::new(128.0, 128.0)),
                ..default()
            },
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
