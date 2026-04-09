use crate::pause::PauseState;
use crate::state::GameState;
use bevy::prelude::*;

fn not_paused(pause: Res<PauseState>) -> bool {
    !pause.paused
}

pub struct CrosshairPlugin;

impl Plugin for CrosshairPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), spawn_crosshair)
            .add_systems(OnExit(GameState::Playing), despawn_crosshair)
            .add_systems(
                Update,
                crosshair_follow_mouse
                    .run_if(in_state(GameState::Playing))
                    .run_if(not_paused),
            );
    }
}

#[derive(Component)]
pub struct Crosshair;

fn spawn_crosshair(mut commands: Commands, mut windows: Query<&mut Window>) {
    // Masquer le curseur système pendant le jeu
    windows.single_mut().cursor.visible = false;

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

fn despawn_crosshair(
    mut commands: Commands,
    query: Query<Entity, With<Crosshair>>,
    mut windows: Query<&mut Window>,
) {
    // Réafficher le curseur système hors du jeu
    windows.single_mut().cursor.visible = true;

    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
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
