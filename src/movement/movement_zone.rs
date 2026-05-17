use bevy::prelude::*;

#[derive(Component, Clone)]
pub struct MovementZone {
    pub margin: Vec2,
    pub on_hit_left: Option<&'static str>,
    pub on_hit_right: Option<&'static str>,
    pub on_hit_top: Option<&'static str>,
    pub on_hit_bottom: Option<&'static str>,
    pub hit_left: bool,
    pub hit_right: bool,
    pub hit_top: bool,
    pub hit_bottom: bool,
}

impl MovementZone {
    pub fn new(margin: Vec2) -> Self {
        Self {
            margin,
            on_hit_left: None,
            on_hit_right: None,
            on_hit_top: None,
            on_hit_bottom: None,
            hit_left: false,
            hit_right: false,
            hit_top: false,
            hit_bottom: false,
        }
    }

    pub fn with_left(mut self, msg: &'static str) -> Self {
        self.on_hit_left = Some(msg);
        self
    }
    pub fn with_right(mut self, msg: &'static str) -> Self {
        self.on_hit_right = Some(msg);
        self
    }
    pub fn with_top(mut self, msg: &'static str) -> Self {
        self.on_hit_top = Some(msg);
        self
    }
    pub fn with_bottom(mut self, msg: &'static str) -> Self {
        self.on_hit_bottom = Some(msg);
        self
    }
}
