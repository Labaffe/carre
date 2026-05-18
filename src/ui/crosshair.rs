use crate::game_manager::difficulty::Difficulty;
use crate::game_manager::state::GameState;
use crate::menu::pause::PauseState;
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
                (crosshair_follow_mouse, crosshair_animate)
                    .run_if(in_state(GameState::Playing))
                    .run_if(not_paused),
            );
    }
}

#[derive(Component)]
pub struct Crosshair;

/// Composant pour l'animation du viseur.
#[derive(Component)]
struct CrosshairAnim {
    elapsed: f32,
}

// ─── Dimensions du viseur ──────────────────────────────────────────

/// Longueur de chaque branche du viseur (px).
const ARM_LENGTH: f32 = 10.0;
/// Epaisseur des branches (px).
const ARM_THICKNESS: f32 = 2.0;
/// Espace entre le centre et le début des branches (px).
const GAP: f32 = 4.0;
/// Epaisseur du contour noir autour de chaque élément (px).
const OUTLINE: f32 = 2.0;
/// Taille du point central (px).
const DOT_SIZE: f32 = 3.0;

fn spawn_crosshair(
    mut commands: Commands,
    windows: Query<&Window>,
    mut cursor_q: Query<&mut bevy::window::CursorOptions>,
) {
    if let Ok(mut cursor) = cursor_q.single_mut() {
        cursor.visible = false;
    }
    let window = windows.single().unwrap();

    let half_h = window.height() / 2.0;
    let start_y = -half_h * 0.5 + 150.0;

    let white = Color::srgba(1.0, 1.0, 1.0, 0.9);
    let black = Color::srgba(0.0, 0.0, 0.0, 0.8);

    // Offset de chaque branche depuis le centre
    let arm_offset = GAP + ARM_LENGTH / 2.0;

    // Données : (offset_x, offset_y, width, height)
    let arms: [(f32, f32, f32, f32); 4] = [
        (0.0, arm_offset, ARM_THICKNESS, ARM_LENGTH),  // haut
        (0.0, -arm_offset, ARM_THICKNESS, ARM_LENGTH), // bas
        (-arm_offset, 0.0, ARM_LENGTH, ARM_THICKNESS), // gauche
        (arm_offset, 0.0, ARM_LENGTH, ARM_THICKNESS),  // droite
    ];

    let parent = commands
        .spawn((
            Transform::from_xyz(0.0, start_y, 10.0),
            Visibility::default(),
            Crosshair,
            CrosshairAnim { elapsed: 0.0 },
        ))
        .id();

    let mut children = Vec::new();

    for (ox, oy, w, h) in arms {
        let outline = commands
            .spawn((
                Sprite { color: black, custom_size: Some(Vec2::new(w + OUTLINE, h + OUTLINE)), ..default() },
                Transform::from_xyz(ox, oy, 0.0),
            ))
            .id();
        children.push(outline);

        let arm = commands
            .spawn((
                Sprite { color: white, custom_size: Some(Vec2::new(w, h)), ..default() },
                Transform::from_xyz(ox, oy, 0.1),
            ))
            .id();
        children.push(arm);
    }

    let dot_outline = commands
        .spawn((
            Sprite { color: black, custom_size: Some(Vec2::splat(DOT_SIZE + OUTLINE)), ..default() },
            Transform::from_xyz(0.0, 0.0, 0.0),
        ))
        .id();
    children.push(dot_outline);

    let dot = commands
        .spawn((
            Sprite { color: white, custom_size: Some(Vec2::splat(DOT_SIZE)), ..default() },
            Transform::from_xyz(0.0, 0.0, 0.1),
        ))
        .id();
    children.push(dot);

    commands.entity(parent).add_children(&children);
}

fn despawn_crosshair(
    mut commands: Commands,
    query: Query<Entity, With<Crosshair>>,
    mut cursor_q: Query<&mut bevy::window::CursorOptions>,
) {
    if let Ok(mut cursor) = cursor_q.single_mut() {
        cursor.visible = true;
    }

    for entity in query.iter() {
        if let Ok(mut e) = commands.get_entity(entity) {
            e.try_despawn();
        }
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
    let Ok((camera, camera_gt)) = camera_q.single() else { return; };

    // Pendant le blocage : téléporter le curseur système sur le crosshair
    if difficulty.elapsed < CROSSHAIR_LOCK_DURATION {
        let Ok((_, crosshair_gt)) = crosshair_q.single() else { return; };
        if let Ok(screen_pos) = camera.world_to_viewport(camera_gt, crosshair_gt.translation()) {
            if let Ok(mut window) = windows.single_mut() {
                window.set_cursor_position(Some(screen_pos));
            }
        }
        return;
    }

    // Après le blocage : le crosshair suit la souris normalement
    let Ok(window) = windows.single() else { return; };
    if let Some(cursor_pos) = window.cursor_position() {
        if let Ok(world_pos) = camera.viewport_to_world_2d(camera_gt, cursor_pos) {
            if let Ok((mut crosshair_transform, _)) = crosshair_q.single_mut() {
                crosshair_transform.translation = world_pos.extend(10.0);
            }
        }
    }
}

// ─── Animation ─────────────────────────────────────────────────────

/// Vitesse de la pulsation (cycles par seconde).
const PULSE_SPEED: f32 = 2.5;
/// Amplitude de la pulsation (± autour de 1.0).
const PULSE_AMPLITUDE: f32 = 0.08;

fn crosshair_animate(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut CrosshairAnim), With<Crosshair>>,
) {
    for (mut transform, mut anim) in query.iter_mut() {
        anim.elapsed += time.delta_secs();
        let scale =
            1.0 + (anim.elapsed * PULSE_SPEED * std::f32::consts::TAU).sin() * PULSE_AMPLITUDE;
        transform.scale = Vec3::splat(scale);
    }
}
