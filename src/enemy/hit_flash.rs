//! Flash visuel temporaire sur un sprite quand l'entité prend un hit.
//!
//! Le composant porte un `Timer` (durée du flash) et une `Color` (la teinte
//! appliquée pendant le flash). Le système `animate_hit_flash` écrit
//! `sprite.color = color` tant que le timer tourne, puis remet
//! `Color::WHITE` et retire le composant.
//!
//! La couleur par défaut (`HitFlash::white`) est un blanc fortement
//! surexposé qui fonctionne sur tous les sprites. Pour un effet custom
//! par entité (boss en rouge, ennemi en jaune, etc.), utiliser
//! `HitFlash::new(duration, color)`.

use bevy::prelude::*;

#[derive(Component)]
pub struct HitFlash {
    pub timer: Timer,
    pub color: Color,
}

impl HitFlash {
    /// Flash blanc surexposé pendant `duration` secondes — preset par défaut.
    pub fn white(duration: f32) -> Self {
        Self {
            timer: Timer::from_seconds(duration, TimerMode::Once),
            color: Color::srgba(100.0, 100.0, 100.0, 1.0),
        }
    }

    /// Flash custom : `duration` secondes, teinte arbitraire.
    pub fn new(duration: f32, color: Color) -> Self {
        Self {
            timer: Timer::from_seconds(duration, TimerMode::Once),
            color,
        }
    }
}

pub fn animate_hit_flash(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Sprite, &mut HitFlash)>,
) {
    for (entity, mut sprite, mut flash) in query.iter_mut() {
        flash.timer.tick(time.delta());

        if flash.timer.is_finished() {
            sprite.color = Color::WHITE;
            commands.entity(entity).remove::<HitFlash>();
        } else {
            sprite.color = flash.color;
        }
    }
}
