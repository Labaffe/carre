use bevy::prelude::*;
use std::time::Duration;

use crate::movement::Movement;

/// Mouvement le long d'une courbe de Bézier quadratique : `start → control → target`
/// parcourue uniformément sur une `duration`.
///
/// La position de départ est capturée à la première évaluation (`current_position`)
/// — pas besoin de la fournir au constructeur, ce qui évite d'avoir à connaître
/// la position du sprite au moment de la construction.
///
/// L'`evaluate` retourne le **delta** position requis pour rejoindre la
/// position cible de la courbe à l'instant `t = at / duration` (clamp [0, 1]).
/// Si l'entité dérive (ex: clamp `MovementZone`), le delta corrige pour
/// retomber sur la courbe — pas d'intégration de vitesse.
#[derive(Clone)]
pub struct Bezier {
    start: Option<Vec2>,
    control: Vec2,
    target: Vec2,
    duration: Duration,
}

impl Bezier {
    pub fn new(control: Vec2, target: Vec2, duration: Duration) -> Self {
        Self {
            start: None,
            control,
            target,
            duration,
        }
    }
}

impl Movement for Bezier {
    fn evaluate(
        &mut self,
        at: Duration,
        _deltatime: Duration,
        current_position: Vec2,
        _velocity: Vec2,
        _player_pos: Vec2,
    ) -> Vec2 {
        let start = *self.start.get_or_insert(current_position);
        let dur = self.duration.as_secs_f32().max(1e-4);
        let t = (at.as_secs_f32() / dur).clamp(0.0, 1.0);
        let one_minus_t = 1.0 - t;
        let bezier_pos = start * (one_minus_t * one_minus_t)
            + self.control * (2.0 * one_minus_t * t)
            + self.target * (t * t);
        bezier_pos - current_position
    }

    fn clone_box(&self) -> Box<dyn Movement + Send + Sync> {
        Box::new(self.clone())
    }
}
