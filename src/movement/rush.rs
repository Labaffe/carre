use bevy::prelude::*;
use crate::movement::Movement;
use std::time::Duration;

/// Rush vers le joueur : à la 1re frame, fige la direction et translate à
/// `speed` constant. Par défaut, direction = vecteur unitaire vers le joueur
/// (diagonale possible).
///
/// `on_axis(axis)` : contraint la direction à ±`axis` selon le côté du joueur
/// au moment du lock. Exemple : `.on_axis(Vec2::X)` donne une charge purement
/// horizontale (gauche ou droite selon où est le joueur).
#[derive(Clone)]
pub struct Rush {
    speed: f32,
    already_set: bool,
    direction: Vec2,
    axis: Option<Vec2>,
}
impl Rush {
    pub fn new(speed: f32) -> Self {
        Rush { speed, already_set: false, direction: Vec2::ZERO, axis: None }
    }
    pub fn on_axis(mut self, axis: Vec2) -> Self {
        self.axis = Some(axis.normalize_or_zero());
        self
    }
}
impl Movement for Rush {
    fn evaluate(
        &mut self,
        at: Duration,
        deltatime: Duration,
        current_position: Vec2,
        velocity: Vec2,
        player_pos: Vec2,
    ) -> Vec2 {
        if !self.already_set {
            let to_player = player_pos - current_position;
            self.direction = match self.axis {
                None => to_player.normalize_or_zero(),
                Some(axis) => {
                    if to_player.dot(axis) >= 0.0 { axis } else { -axis }
                }
            };
            self.already_set = true;
        }
        self.direction * self.speed * deltatime.as_secs_f32()
    }
    fn clone_box(&self) -> Box<dyn Movement + Send + Sync> {
        Box::new(self.clone())
    }
}