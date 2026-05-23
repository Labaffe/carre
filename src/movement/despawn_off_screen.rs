use bevy::prelude::*;
use crate::GameState;
use crate::movement::bounding_radius::BoundingRadius;

pub struct DespawnOffScreenPlugin;

impl Plugin for DespawnOffScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            despawn_off_screen.run_if(in_state(GameState::Playing)),
        );
    }
}

/// Marker : despawn l'entité dès qu'elle est entièrement sortie de l'écran.
///
/// Bornes recalculées chaque frame depuis le `Window` (suit le resize), en
/// `physical_*` pour rester cohérent avec la caméra 2D. Une tolérance fixe
/// (`MARGIN_X` / `MARGIN_Y`) laisse le sprite quitter l'écran proprement.
/// Avec un `BoundingRadius`, le despawn n'a lieu que quand le sprite est
/// complètement hors écran.
#[derive(Component)]
pub struct DespawnOffScreen;

/// Tolérance hors-écran avant despawn, en fraction de l'écran.
const MARGIN_X: f32 = 0.1;
const MARGIN_Y: f32 = 0.25;

fn despawn_off_screen(
    mut commands: Commands,
    window: Single<&Window>,
    query: Query<(Entity, &Transform, Option<&BoundingRadius>), With<DespawnOffScreen>>,
) {
    let w = window.physical_width() as f32;
    let h = window.physical_height() as f32;

    let min_x = -(0.5 + MARGIN_X) * w;
    let max_x = (0.5 + MARGIN_X) * w;
    let min_y = -(0.5 + MARGIN_Y) * h;
    let max_y = (0.5 + MARGIN_Y) * h;

    for (entity, transform, bounding) in &query {
        let r = bounding.map(|b| b.0).unwrap_or(0.0);
        let pos = transform.translation;
        let fully_outside = pos.x + r < min_x
            || pos.x - r > max_x
            || pos.y + r < min_y
            || pos.y - r > max_y;

        if fully_outside {
            // try_despawn : ne panique pas si l'entité a déjà été despawnée
            // ce tick par un autre système (ex: DespawnSelf en fin d'animation
            // de mort sur une entité aussi hors écran).
            if let Ok(mut e) = commands.get_entity(entity) {
                e.try_despawn();
            }
        }
    }
}
