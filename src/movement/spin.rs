//! Rotation continue de l'entité autour de son propre axe Z.
//!
//! Pendant tout le temps où le composant [`Spin`] est présent sur une entité,
//! le système [`spin_driver`] applique `angular_velocity * dt` à la rotation
//! du `Transform`. Au retrait du composant, si `auto_reset = true`, la rotation
//! est ramenée à `Quat::IDENTITY` via un hook `on_remove` du framework Bevy.

use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;

#[derive(Component, Clone)]
#[component(on_remove = spin_on_remove)]
pub struct Spin {
    /// Vitesse de rotation en radians/seconde (positif = anti-horaire).
    pub angular_velocity: f32,
    /// Si `true`, la rotation est remise à `Quat::IDENTITY` au retrait du
    /// composant (typique pour des phases temporaires : charge boss, etc.).
    pub auto_reset: bool,
}

impl Spin {
    pub fn new(angular_velocity: f32) -> Self {
        Self { angular_velocity, auto_reset: false }
    }
    pub fn with_auto_reset(mut self) -> Self {
        self.auto_reset = true;
        self
    }
}

fn spin_on_remove(mut world: DeferredWorld, ctx: HookContext) {
    let auto_reset = world
        .get::<Spin>(ctx.entity)
        .map(|s| s.auto_reset)
        .unwrap_or(false);
    if auto_reset {
        if let Some(mut transform) = world.get_mut::<Transform>(ctx.entity) {
            transform.rotation = Quat::IDENTITY;
        }
    }
}

pub fn spin_driver(time: Res<Time>, mut query: Query<(&mut Transform, &Spin)>) {
    let dt = time.delta_secs();
    for (mut transform, spin) in &mut query {
        transform.rotate_z(spin.angular_velocity * dt);
    }
}
