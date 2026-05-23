//! Screen shake — secousse de caméra déclenchée via `ScreenShakeEvent`.
//!
//! Architecture :
//! - `ScreenShakeEvent` (Event) est trigger par les systèmes qui veulent du
//!   feedback visuel (player hit, bombe, mort boss, AOE mine/kamikaze).
//! - Un observer global pose un composant `ScreenShake { timer, initial_intensity, base }`
//!   sur la caméra. La `base` est la position originale, restaurée à la fin.
//! - Le système `screen_shake_update` écrit chaque frame un offset random
//!   ABSOLU (pas additif) dans `transform.translation = base + random*amp`,
//!   évitant le drift d'un random walk additif.
//! - L'amplitude suit une enveloppe `(1 - progress)²` qui fade out doucement.
//!
//! Gestion des triggers superposés : un nouveau shake remplace le courant
//! seulement si son intensité ≥ amplitude COURANTE (fadée). Évite qu'un petit
//! hit interrompe une grosse explosion en cours. La base est conservée.

use bevy::prelude::*;
use std::time::Duration;

pub struct ScreenShakePlugin;

impl Plugin for ScreenShakePlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_screen_shake_event)
            .add_systems(Update, screen_shake_update);
    }
}

#[derive(Event, Debug, Clone, Copy)]
pub struct ScreenShakeEvent {
    pub intensity: f32,
    pub duration: f32,
}

impl ScreenShakeEvent {
    pub const PLAYER_HIT: Self = Self { intensity: 8.0, duration: 0.25 };
    pub const MINE: Self = Self { intensity: 12.0, duration: 0.4 };
    pub const KAMIKAZE: Self = Self { intensity: 14.0, duration: 0.4 };
    pub const BOMB: Self = Self { intensity: 22.0, duration: 0.7 };
    pub const BOSS_DEATH: Self = Self { intensity: 28.0, duration: 1.4 };
}

#[derive(Component)]
struct ScreenShake {
    timer: Timer,
    initial_intensity: f32,
    base: Vec3,
}

fn on_screen_shake_event(
    trigger: On<ScreenShakeEvent>,
    mut commands: Commands,
    camera_q: Query<(Entity, &Transform, Option<&ScreenShake>), With<Camera2d>>,
) {
    let ev = *trigger.event();
    let Ok((entity, transform, existing)) = camera_q.single() else { return; };

    let (base, should_replace) = match existing {
        None => (transform.translation, true),
        Some(s) => {
            let progress = s.timer.fraction();
            let current_amp = s.initial_intensity * (1.0 - progress).powi(2);
            (s.base, ev.intensity >= current_amp)
        }
    };

    if !should_replace { return; }

    if let Ok(mut e) = commands.get_entity(entity) {
        e.try_insert(ScreenShake {
            timer: Timer::new(Duration::from_secs_f32(ev.duration), TimerMode::Once),
            initial_intensity: ev.intensity,
            base,
        });
    }
}

fn screen_shake_update(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut ScreenShake, &mut Transform)>,
) {
    for (entity, mut shake, mut transform) in q.iter_mut() {
        shake.timer.tick(time.delta());
        if shake.timer.is_finished() {
            transform.translation = shake.base;
            if let Ok(mut e) = commands.get_entity(entity) {
                e.try_remove::<ScreenShake>();
            }
            continue;
        }
        let progress = shake.timer.fraction();
        let envelope = (1.0 - progress).powi(2);
        let amp = shake.initial_intensity * envelope;
        let dx = (fastrand::f32() - 0.5) * 2.0 * amp;
        let dy = (fastrand::f32() - 0.5) * 2.0 * amp;
        transform.translation.x = shake.base.x + dx;
        transform.translation.y = shake.base.y + dy;
    }
}
