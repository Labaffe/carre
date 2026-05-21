use bevy::prelude::*;
use std::time::Duration;

use crate::movement::Movement;

/// Mouvement le long d'une courbe de Bézier quadratique : `start → control → target`
/// parcourue uniformément sur une `duration`.
///
/// Helper [`Bezier::passing_through`] : calcule le control pour qu'à `t=0.5`
/// la courbe passe **exactement** par un point `pass_through` désiré
/// (formule `control = 2·P − (start+target)/2`).
///
/// L'`evaluate` retourne le **delta** position requis pour rejoindre la
/// position cible de la courbe à l'instant `t = at / duration` (clamp [0, 1]).
/// Si l'entité dérive (ex: clamp `MovementZone`), le delta corrige pour
/// retomber sur la courbe — pas d'intégration de vitesse.
#[derive(Clone)]
pub struct Bezier {
    start: Vec2,
    control: Vec2,
    target: Vec2,
    duration: Duration,
}

impl Bezier {
    pub fn new(start: Vec2, control: Vec2, target: Vec2, duration: Duration) -> Self {
        Self {
            start,
            control,
            target,
            duration,
        }
    }

    /// Construit une Bézier qui passe par `pass_through` exactement à `t=0.5`.
    pub fn passing_through(
        start: Vec2,
        pass_through: Vec2,
        target: Vec2,
        duration: Duration,
    ) -> Self {
        let control = 2.0 * pass_through - (start + target) * 0.5;
        Self::new(start, control, target, duration)
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
        let dur = self.duration.as_secs_f32().max(1e-4);
        let t = (at.as_secs_f32() / dur).clamp(0.0, 1.0);
        let one_minus_t = 1.0 - t;
        let bezier_pos = self.start * (one_minus_t * one_minus_t)
            + self.control * (2.0 * one_minus_t * t)
            + self.target * (t * t);
        bezier_pos - current_position
    }

    fn clone_box(&self) -> Box<dyn Movement + Send + Sync> {
        Box::new(self.clone())
    }
}
