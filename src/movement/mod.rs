pub mod movement;
pub mod movements;
pub mod sinusoid;
pub mod translate;
pub mod chase;
pub mod rush;
pub mod goto;
pub mod shake;
pub mod rotate;
pub mod oscilate;
pub mod movement_zone;
pub mod bounding_radius;
use bevy::prelude::*;
use crate::behavior::choice_list::TransitionMessages;
use crate::movement::bounding_radius::BoundingRadius;
use crate::movement::movement_zone::MovementZone;
use crate::movement::movements::Movements;
use crate::GameState;
use crate::movement::movement::Movement;
use crate::player::player::Player;
pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_systems(
            FixedUpdate,
            movement_driver.run_if(in_state(GameState::Playing))
        );
    }
}

pub fn movement_driver(
    time: Res<Time>,
    windows: Query<&Window>,
    mut query: Query<(
        Entity,
        &mut Movements,
        &mut Transform,
        Option<&mut MovementZone>,
        Option<&mut TransitionMessages>,
        Option<&BoundingRadius>,
    )>,
    query_player: Query<(&Player, &Transform), Without<Movements>>,
) {
    // Gracefully handle missing or multiple players
    let Ok((_, player_transform)) = query_player.single() else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };

    let player_pos = player_transform.translation.xy();

    for (_, mut movements, mut transform, zone, messages, bounding) in query.iter_mut() {
        let movement = movements.get_movement(
            transform.translation.xy(),
            time.delta(),
            player_pos,
        );
        let mut next = transform.translation.xy() + movement;

        if let Some(mut zone) = zone {
            let r = bounding.map(|b| b.0).unwrap_or(0.0);
            // Utiliser physical_size (world units) plutôt que logical_size
            // (qui dépend du scale factor / DPI) — la caméra 2D rend en physical.
            let w = window.physical_width() as f32;
            let h = window.physical_height() as f32;
            let min_x = (zone.margin.x - 0.5) * w + r;
            let max_x = (0.5 - zone.margin.x) * w - r;
            let min_y = (zone.margin.y - 0.5) * h + r;
            let max_y = (0.5 - zone.margin.y) * h - r;

            let hits_left = next.x <= min_x;
            let hits_right = next.x >= max_x;
            let hits_bottom = next.y <= min_y;
            let hits_top = next.y >= max_y;

            if let Some(mut msgs) = messages {
                if hits_left && !zone.hit_left {
                    if let Some(m) = zone.on_hit_left {
                        msgs.messages.push(m.to_string());
                    }
                }
                if hits_right && !zone.hit_right {
                    if let Some(m) = zone.on_hit_right {
                        msgs.messages.push(m.to_string());
                    }
                }
                if hits_top && !zone.hit_top {
                    if let Some(m) = zone.on_hit_top {
                        msgs.messages.push(m.to_string());
                    }
                }
                if hits_bottom && !zone.hit_bottom {
                    if let Some(m) = zone.on_hit_bottom {
                        msgs.messages.push(m.to_string());
                    }
                }
            }
            zone.hit_left = hits_left;
            zone.hit_right = hits_right;
            zone.hit_top = hits_top;
            zone.hit_bottom = hits_bottom;

            next.x = next.x.clamp(min_x, max_x);
            next.y = next.y.clamp(min_y, max_y);
        }

        transform.translation.x = next.x;
        transform.translation.y = next.y;
    }
}
