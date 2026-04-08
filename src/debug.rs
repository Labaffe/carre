use bevy::prelude::*;
use crate::asteroid::Asteroid;
use crate::collision::PLAYER_RADIUS;
use crate::player::Player;

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DebugMode(false))
            .add_systems(Update, (toggle_debug, draw_hitboxes));
    }
}

#[derive(Resource)]
pub struct DebugMode(pub bool);

fn toggle_debug(keyboard: Res<ButtonInput<KeyCode>>, mut debug: ResMut<DebugMode>) {
    if keyboard.just_pressed(KeyCode::F1) {
        debug.0 = !debug.0;
        println!("Debug mode : {}", if debug.0 { "ON" } else { "OFF" });
    }
}

fn draw_hitboxes(
    debug: Res<DebugMode>,
    mut gizmos: Gizmos,
    player_q: Query<&Transform, With<Player>>,
    asteroid_q: Query<(&Transform, &Asteroid)>,
) {
    if !debug.0 {
        return;
    }

    // hitbox du joueur (vert)
    for transform in player_q.iter() {
        gizmos.circle_2d(
            transform.translation.truncate(),
            PLAYER_RADIUS,
            Color::GREEN,
        );
    }

    // hitbox des astéroïdes (rouge)
    for (transform, asteroid) in asteroid_q.iter() {
        gizmos.circle_2d(
            transform.translation.truncate(),
            asteroid.radius,
            Color::RED,
        );
    }
}
