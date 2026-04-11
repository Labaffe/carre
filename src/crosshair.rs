use crate::difficulty::Difficulty;
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
    let mut window = windows.single_mut();
    // Masquer le curseur système pendant le jeu
    window.cursor.visible = false;

    let half_h = window.height() / 2.0;
    // Position initiale : juste devant le vaisseau (au-dessus)
    let start_y = -half_h * 0.5 + 150.0;

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
        .spawn((
            SpatialBundle {
                transform: Transform::from_xyz(0.0, start_y, 1.0),
                ..default()
            },
            Crosshair,
        ))
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
        if let Some(e) = commands.get_entity(entity) { e.despawn_recursive(); }
    }
}

/// Durée du blocage initial du crosshair (secondes).
const CROSSHAIR_LOCK_DURATION: f32 = 0.4;

fn crosshair_follow_mouse(
    mut windows: Query<&mut Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut crosshair_q: Query<(&mut Transform, &GlobalTransform), With<Crosshair>>,
    difficulty: Res<Difficulty>,
) {
    let (camera, camera_gt) = camera_q.single();

    // Pendant le blocage : téléporter le curseur système sur le crosshair
    if difficulty.elapsed < CROSSHAIR_LOCK_DURATION {
        let (_, crosshair_gt) = crosshair_q.single();
        if let Some(screen_pos) = camera.world_to_viewport(camera_gt, crosshair_gt.translation()) {
            windows.single_mut().set_cursor_position(Some(screen_pos));
        }
        return;
    }

    // Après le blocage : le crosshair suit la souris normalement
    let window = windows.single();
    if let Some(cursor_pos) = window.cursor_position() {
        if let Some(world_pos) = camera.viewport_to_world_2d(camera_gt, cursor_pos) {
            let (mut crosshair_transform, _) = crosshair_q.single_mut();
            crosshair_transform.translation = world_pos.extend(1.0);
        }
    }
}
