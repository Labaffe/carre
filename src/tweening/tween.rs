use crate::tweening::component::{TweenTarget};
use crate::tweening::easing::{Ease};
use bevy::prelude::*;
#[derive(Clone)]
pub struct Tween {
    pub start: f32,
    pub end: f32,
    pub duration: f32,
    pub elapsed: f32,
    pub ease: Ease,
}

impl Tween {
    pub fn new(start: f32, end: f32, duration: f32, ease: Ease) -> Self {
        Self {
            start,
            end,
            duration,
            elapsed: 0.0,
            ease,
        }
    }

    pub fn value(&self) -> f32 {
        let t = (self.elapsed / self.duration).clamp(0.0, 1.0);
        let t = self.ease.sample(t);
        self.start + (self.end - self.start) * t
    }

    pub fn tick(&mut self, dt: f32) {
        self.elapsed += dt;
    }

    pub fn finished(&self) -> bool {
        self.elapsed >= self.duration
    }
}

#[derive(Component)]
pub struct TweenSequence<T: TweenTarget> {
    pub current: Tween,
    pub queue: Vec<Tween>,
    pub _marker: std::marker::PhantomData<T>,
}

impl<T: TweenTarget> TweenSequence<T> {
    pub fn new(tween: Tween) -> Self {
        Self {
            current: tween,
            queue: Vec::new(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn then(mut self, tween: Tween) -> Self {
        self.queue.push(tween);
        self
    }
}