use bevy::prelude::*;
use crate::asteroid::Asteroid;
use crate::collision::PLAYER_RADIUS;
use crate::difficulty::Difficulty;
use crate::missile::Missile;
use crate::player::Player;

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DebugMode(false))
            .add_systems(Startup, setup_debug_ui)
            .add_systems(Update, (toggle_debug, draw_hitboxes, update_debug_ui));
    }
}

#[derive(Resource)]
pub struct DebugMode(pub bool);

#[derive(Component)]
struct DebugUI;

fn setup_debug_ui(mut commands: Commands) {
    commands.spawn((
        TextBundle {
            text: Text::from_sections([
                TextSection::new("", TextStyle { font_size: 16.0, color: Color::WHITE, ..default() }),
            ]),
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                left: Val::Px(10.0),
                ..default()
            },
            visibility: Visibility::Hidden,
            ..default()
        },
        DebugUI,
    ));
}

fn toggle_debug(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut debug: ResMut<DebugMode>,
    mut ui_q: Query<&mut Visibility, With<DebugUI>>,
) {
    if keyboard.just_pressed(KeyCode::F1) {
        debug.0 = !debug.0;
        if let Ok(mut vis) = ui_q.get_single_mut() {
            *vis = if debug.0 { Visibility::Visible } else { Visibility::Hidden };
        }
    }
}

fn update_debug_ui(
    debug: Res<DebugMode>,
    time: Res<Time>,
    difficulty: Res<Difficulty>,
    mut ui_q: Query<&mut Text, With<DebugUI>>,
) {
    if !debug.0 { return; }

    let fps = 1.0 / time.delta_seconds();
    let elapsed = difficulty.elapsed;
    let factor = difficulty.factor;

    let minutes = (elapsed / 60.0) as u32;
    let seconds = (elapsed % 60.0) as u32;

    if let Ok(mut text) = ui_q.get_single_mut() {
        text.sections[0].value = format!(
            "[DEBUG]\nFPS        : {:.0}\nTimer      : {:02}:{:02}\nDifficulté : x{:.2}",
            fps, minutes, seconds, factor
        );
    }
}

fn draw_hitboxes(
    debug: Res<DebugMode>,
    mut gizmos: Gizmos,
    player_q: Query<&Transform, With<Player>>,
    asteroid_q: Query<(&Transform, &Asteroid)>,
    missile_q: Query<(&Transform, &Missile)>,
) {
    if !debug.0 { return; }

    for transform in player_q.iter() {
        gizmos.circle_2d(transform.translation.truncate(), PLAYER_RADIUS, Color::GREEN);
    }

    for (transform, asteroid) in asteroid_q.iter() {
        gizmos.circle_2d(transform.translation.truncate(), asteroid.radius, Color::RED);
    }

    for (transform, missile) in missile_q.iter() {
        let pos = transform.translation.truncate();
        let angle = transform.rotation.to_euler(EulerRot::ZYX).0;
        let cos = angle.cos();
        let sin = angle.sin();
        // Axes locaux : X local = (cos, sin), Y local = (-sin, cos)
        let ax = Vec2::new(cos, sin);
        let ay = Vec2::new(-sin, cos);

        let hw = missile.half_width;
        let hl = missile.half_length;

        let corners = [
            pos + ax * hw + ay * hl,
            pos - ax * hw + ay * hl,
            pos - ax * hw - ay * hl,
            pos + ax * hw - ay * hl,
        ];

        for i in 0..4 {
            gizmos.line_2d(corners[i], corners[(i + 1) % 4], Color::YELLOW);
        }
    }
}
