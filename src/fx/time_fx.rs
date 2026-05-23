//! TimeFx — slow-motion et hit-stop via `Time<Virtual>::set_relative_speed`.
//!
//! Un Event `TimeFxEvent { scale, duration }` change le relative_speed du
//! `Time<Virtual>` pour `duration` secondes. Le timer interne tick en
//! `Time<Real>` (sinon le slow-mo se prolonge proportionnellement à sa propre
//! échelle — bug évident). À la fin de l'effet, restore `1.0`.
//!
//! Tous les systèmes utilisant `Res<Time>` (= Time<Virtual>) ralentissent en
//! cohérence : mouvement, animations, AI, projectiles, combo timer. Comme les
//! effets durent < 1s, l'impact sur les timers UI est négligeable.
//!
//! Superposition : un nouveau trigger REMPLACE l'effet courant — choix simple
//! voulu (pas de stack à gérer). Pour chaîner hit-stop + slow-mo, faire deux
//! triggers côté caller.

use bevy::prelude::*;

pub struct TimeFxPlugin;

impl Plugin for TimeFxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TimeFxState>()
            .add_observer(on_time_fx_event)
            .add_systems(Update, time_fx_tick);
    }
}

#[derive(Event, Debug, Clone, Copy)]
pub struct TimeFxEvent {
    /// Multiplicateur de vitesse. 0.0 = freeze total (hit-stop),
    /// 0.15–0.3 = slow-mo prononcé, 1.0 = normal.
    pub scale: f32,
    /// Durée en secondes réelles (Time<Real>).
    pub duration: f32,
}

impl TimeFxEvent {
    pub const HIT_STOP_PLAYER: Self = Self { scale: 0.0, duration: 0.04 };
    pub const SLOWMO_PLAYER_DEATH: Self = Self { scale: 0.15, duration: 0.6 };
    pub const SLOWMO_BOSS_KILL: Self = Self { scale: 0.25, duration: 0.8 };
    pub const SLOWMO_BOMB_CLUTCH: Self = Self { scale: 0.3, duration: 0.45 };
}

#[derive(Resource, Default)]
struct TimeFxState {
    /// Secondes réelles restantes avant restauration. 0 = aucun effet en cours.
    remaining_real: f32,
}

fn on_time_fx_event(
    trigger: On<TimeFxEvent>,
    mut state: ResMut<TimeFxState>,
    mut time: ResMut<Time<Virtual>>,
) {
    let ev = *trigger.event();
    time.set_relative_speed(ev.scale.max(0.0));
    state.remaining_real = ev.duration;
}

fn time_fx_tick(
    real: Res<Time<Real>>,
    mut state: ResMut<TimeFxState>,
    mut time: ResMut<Time<Virtual>>,
) {
    if state.remaining_real <= 0.0 {
        return;
    }
    state.remaining_real -= real.delta_secs();
    if state.remaining_real <= 0.0 {
        state.remaining_real = 0.0;
        time.set_relative_speed(1.0);
    }
}
