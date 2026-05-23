//! Détection du joueur dans une zone géométrique attachée à une entité.
//!
//! Mirror perceptif de [`crate::movement::movement_zone::MovementZone`] :
//! sur transition entrée/sortie du joueur dans la zone, pousse un message
//! dans le `TransitionMessages` de l'entité porteuse (pour piloter une
//! transition de phase via le `BehaviorComponent`).
//!
//! La forme est une [`Shape`] orientée par la `Transform` de l'entité, donc
//! les `Rect` suivent la rotation. La détection se fait contre un cercle de
//! rayon `PLAYER_RADIUS` autour du joueur.

use bevy::prelude::*;

use crate::behavior::choice_list::TransitionMessages;
use crate::game_manager::state::GameState;
use crate::geometry::shape::{shape_hits_circle, Shape};
use crate::physic::collision::PLAYER_RADIUS;
use crate::player::player::Player;

pub struct PlayerDetectionPlugin;

impl Plugin for PlayerDetectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            detect_player.run_if(in_state(GameState::Playing)),
        );
    }
}

#[derive(Component, Clone)]
pub struct PlayerDetection {
    pub shape: Shape,
    pub on_enter: Option<&'static str>,
    pub on_exit: Option<&'static str>,
    pub inside: bool,
    /// Cooldown (secondes) après chaque `on_enter` qui a poussé : tant que
    /// `cooldown_remaining > 0`, les rising edges ne pushent rien. 0.0 = pas
    /// de cooldown (default). `on_exit` n'est pas affecté.
    pub cooldown_duration: f32,
    /// Temps restant avant de pouvoir re-fire `on_enter`. Décrémenté chaque
    /// frame, mis à `cooldown_duration` quand `on_enter` fire.
    pub cooldown_remaining: f32,
}

fn detect_player(
    time: Res<Time>,
    player_transform: Single<&Transform, With<Player>>,
    mut query: Query<(
        &Transform,
        &mut PlayerDetection,
        Option<&mut TransitionMessages>,
    )>,
) {
    let player_pos = player_transform.translation.truncate();
    let dt = time.delta_secs();

    for (transform, mut detection, messages) in query.iter_mut() {
        if detection.cooldown_remaining > 0.0 {
            detection.cooldown_remaining = (detection.cooldown_remaining - dt).max(0.0);
        }

        let inside_now = shape_hits_circle(
            transform.translation.truncate(),
            transform.rotation,
            &detection.shape,
            player_pos,
            PLAYER_RADIUS,
        );

        if let Some(mut msgs) = messages {
            if inside_now && !detection.inside && detection.cooldown_remaining <= 0.0 {
                if let Some(m) = detection.on_enter {
                    msgs.messages.push(m.to_string());
                    detection.cooldown_remaining = detection.cooldown_duration;
                }
            }
            if !inside_now && detection.inside {
                if let Some(m) = detection.on_exit {
                    msgs.messages.push(m.to_string());
                }
            }
        }

        detection.inside = inside_now;
    }
}
