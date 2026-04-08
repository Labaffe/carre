use bevy::prelude::*;
use fastrand;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                movement,
                crosshair_follow_mouse,
                rotate_towards_crosshair,
                spawn_asteroids,
                move_asteroids,
                scroll_background,
            ),
        )
        .run();
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Background;

#[derive(Component)]
struct Crosshair;

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

fn setup(mut commands: Commands, mut windows: Query<&mut Window>, asset_server: Res<AssetServer>) {
    // masquer le curseur système
    windows.single_mut().cursor.visible = false;

    // musique de fond en boucle
    commands.spawn(AudioBundle {
        source: asset_server.load("gradius.ogg"),
        settings: PlaybackSettings::LOOP,
    });

    // caméra
    commands.spawn(Camera2dBundle::default());

    // fond d'espace scrollant : deux copies empilées verticalement
    let bg = asset_server.load("space_background.png");
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

    // initialiser le spawner d'astéroides
    commands.insert_resource(AsteroidSpawner::default());

    // joueur (vaisseau)
    commands.spawn((
        SpriteBundle {
            texture: asset_server.load("vaisseau.png"),
            sprite: Sprite {
                custom_size: Some(Vec2::new(64.0, 64.0)),
                ..default()
            },
            ..default()
        },
        Player,
    ));

    // réticule (croix rouge) : barre horizontale + verticale enfants d'un parent
    let h_bar = commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(1.0, 0.0, 0.0),
                custom_size: Some(Vec2::new(10.0, 2.0)),
                ..default()
            },
            ..default()
        })
        .id();

    let v_bar = commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(1.0, 0.0, 0.0),
                custom_size: Some(Vec2::new(2.0, 10.0)),
                ..default()
            },
            ..default()
        })
        .id();

    commands
        .spawn((SpatialBundle::default(), Crosshair))
        .push_children(&[h_bar, v_bar]);
}

fn scroll_background(mut query: Query<&mut Transform, With<Background>>, time: Res<Time>) {
    let speed = 150.0;
    let image_height = 1536.0;

    for mut transform in query.iter_mut() {
        transform.translation.y -= speed * time.delta_seconds();

        // quand une copie sort en bas, on la remet au-dessus de l'autre
        if transform.translation.y <= -image_height {
            transform.translation.y += image_height * 2.0;
        }
    }
}

fn movement(keyboard: Res<ButtonInput<KeyCode>>, mut query: Query<&mut Transform, With<Player>>) {
    let mut transform = query.single_mut();
    let speed = 200.0;
    let mut direction = Vec3::ZERO;

    if keyboard.pressed(KeyCode::KeyW) {
        direction.y += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        direction.y -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        direction.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        direction.x += 1.0;
    }

    transform.translation += direction * speed * 0.016;
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
        let x = fastrand::f32() * window.width() - window.width() / 2.0; // position aléatoire en X entre -800 et +800
        let y = 500.0; // haut de l'écran

        // lance une piece, si pile, spawne un astéroïde rapide et petit (asteriod_1.png 24 par 24 pixels)
        // sinon un asterioïde lent et grand (asteroid_2.png 41 par 41 pixels)

        let fastrand = fastrand::bool();
        let image: &str;
        let sprite;
        let velocity: Vec3;

        // affichage du sprit dans une rotation aléatoire a ajouté dans la transformation du sprite
        let transform = Transform::from_xyz(x, y, 0.0).with_rotation(Quat::from_rotation_z(
            fastrand::f32() * std::f32::consts::TAU,
        ));

        if fastrand {
            image = "asteroid_1.png";
            sprite = Sprite {
                custom_size: Some(Vec2::new(24.0, 24.0)),
                ..default()
            };
            velocity = Vec3::new(0.0, -300.0, 0.0); // descend à 300 unités/sec
            println!("Astéroïde rapide et petit");
        } else {
            image = "asteroid_2.png";
            sprite = Sprite {
                custom_size: Some(Vec2::new(41.0, 41.0)),
                ..default()
            };
            velocity = Vec3::new(0.0, -100.0, 0.0); // descend à 300 unités/sec
            println!("Astéroïde lent et grand");
        }

        commands.spawn((
            SpriteBundle {
                texture: asset_server.load(image),
                sprite: sprite,
                transform: transform,
                ..default()
            },
            Asteroid { velocity: velocity },
        ));
    }
}

fn move_asteroids(mut query: Query<(&mut Transform, &Asteroid)>, time: Res<Time>) {
    for (mut transform, asteroid) in query.iter_mut() {
        transform.translation += asteroid.velocity * time.delta_seconds();

        // supprimer l'astéroïde s'il sort de l'écran
        if transform.translation.y < -500.0 {
            // sera géré avec un système de despawn plus tard
        }
    }
}
