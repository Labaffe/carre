use bevy::prelude::*;

pub fn card_center_x(i:i32,window:&Window,count:usize)->f32 {
    let center_x = window.width() / 2.0;
    let spacing = 250.0;
    center_x + ((i as f32) - (count as f32) * 0.5) * spacing
}