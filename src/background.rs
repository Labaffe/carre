//! Background spatial scrollant en boucle.
//! Deux copies de l'image se suivent verticalement pour un défilement infini.
//! La vitesse de scroll est proportionnelle au carré du facteur de difficulté.
//! Spawné à l'entrée du Playing, caché au game over, réaffiché au restart.

use crate::difficulty::Difficulty;
use crate::state::GameState;
use bevy::prelude::*;

pub struct BackgroundPlugin;

impl Plugin for BackgroundPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), setup_background)
            .add_systems(
                Update,
                scroll_background.run_if(in_state(GameState::Playing)),
            );
    }
}

#[derive(Component)]
pub struct Background;

fn setup_background(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    existing: Query<Entity, With<Background>>,
) {
    // Ne pas re-spawner si le background existe déjà (cas du restart après game over)
    if !existing.is_empty() {
        // Réafficher les backgrounds cachés
        return;
    }

    let bg = asset_server.load("images/space_background.png");

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
}

/// Fait défiler les deux sprites de background vers le bas.
/// Quand un sprite sort de l'écran, il est repositionné au-dessus de l'autre.
fn scroll_background(
    mut query: Query<&mut Transform, With<Background>>,
    time: Res<Time>,
    difficulty: Res<Difficulty>,
) {
    let base_speed = 150.0;
    let image_height = 1536.0;
    // Scaling logarithmique : gros boost au passage à 3, puis accélérations
    // de plus en plus discrètes. ln(1)=0, ln(3)≈1.1, ln(5)≈1.6, ln(7)≈1.9, ln(9)≈2.2
    let speed = base_speed * (1.0 + difficulty.factor * 3.0);

    for mut transform in query.iter_mut() {
        transform.translation.y -= speed * time.delta_seconds();

        if transform.translation.y <= -image_height {
            transform.translation.y += image_height * 2.0;
        }
    }
}
