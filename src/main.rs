use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (movement, crosshair_follow_mouse))
        .run();
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Crosshair;

fn setup(mut commands: Commands, mut windows: Query<&mut Window>, asset_server: Res<AssetServer>) {
    // masquer le curseur système
    windows.single_mut().cursor.visible = false;

    // caméra
    commands.spawn(Camera2dBundle::default());

    // joueur (vaisseau)
    commands.spawn((
        SpriteBundle {
            texture: asset_server.load("vaisseau.png"),
            ..default()
        },
        Player,
    ));

    // réticule (croix rouge) : barre horizontale + verticale enfants d'un parent
    let h_bar = commands.spawn(SpriteBundle {
        sprite: Sprite {
            color: Color::rgb(1.0, 0.0, 0.0),
            custom_size: Some(Vec2::new(10.0, 2.0)),
            ..default()
        },
        ..default()
    }).id();

    let v_bar = commands.spawn(SpriteBundle {
        sprite: Sprite {
            color: Color::rgb(1.0, 0.0, 0.0),
            custom_size: Some(Vec2::new(2.0, 10.0)),
            ..default()
        },
        ..default()
    }).id();

    commands.spawn((SpatialBundle::default(), Crosshair))
        .push_children(&[h_bar, v_bar]);
}

fn movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<Player>>,
) {
    let mut transform = query.single_mut();
    let speed = 200.0;
    let mut direction = Vec3::ZERO;

    if keyboard.pressed(KeyCode::ArrowUp)    { direction.y += 1.0; }
    if keyboard.pressed(KeyCode::ArrowDown)  { direction.y -= 1.0; }
    if keyboard.pressed(KeyCode::ArrowLeft)  { direction.x -= 1.0; }
    if keyboard.pressed(KeyCode::ArrowRight) { direction.x += 1.0; }

    transform.translation += direction * speed * 0.016;
}

fn crosshair_follow_mouse(
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut crosshair_q: Query<&mut Transform, With<Crosshair>>,
) {
    let window = windows.single();
    let (camera, camera_transform) = camera_q.single();

    if let Some(cursor_pos) = window.cursor_position() {
        if let Some(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
            crosshair_q.single_mut().translation = world_pos.extend(1.0);
        }
    }
}
