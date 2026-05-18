use bevy::prelude::*;
use crate::movement::Movement;
use std::time::Duration;
#[derive(Clone)]
pub struct Oscilate {
    direction:Vec2,
    origin:Vec2,
    normal:Vec2,
    speed:f32,    
    freq:f32,
    amplitude:f32,
    already_set:bool
}
impl Oscilate {
    pub fn new(
        direction:Vec2,
        origin:Vec2,
        freq:f32,
        amplitude:f32
    )-> Self {
        Self {
            direction,
            origin,
            normal:Vec2::new(-direction.y,direction.x),
            speed:0.0,
            freq,
            amplitude
            ,already_set:false
        }
    }
    fn local_value(&self,current_pos:Vec2)->f32 {
        let local_pos = current_pos-self.origin;
        local_pos.dot(self.normal)
    }
    fn set_speed_from_pos(&mut self,value:f32,amplitude:f32) {
        // Oscillateur harmonique : dv/dt = -freq*x ⇒ ω = √freq.
        // Pour atteindre l'amplitude max A depuis la position initiale x₀ :
        //   v₀ = ω * √(A² − x₀²)
        // (le facteur ω était manquant dans l'ancienne formule, ce qui réduisait
        // l'amplitude effective d'un facteur 1/√freq.)
        let omega = self.freq.sqrt();
        self.speed = -omega * (amplitude * amplitude - value * value).max(0.0).sqrt();
    }
}
impl Movement for Oscilate {
    fn evaluate(
        &mut self,
        at:Duration,
        deltatime:Duration,
        current_position:Vec2,
        velocity:Vec2,
        player_pos:Vec2
    )->Vec2 { 
        let value = self.local_value(current_position);
        if !self.already_set {
            if value > self.amplitude {
                self.speed = -1000.0;
            }
            else if value < -self.amplitude {
                self.speed = 1000.0;
            }
            else {
                self.set_speed_from_pos(value, self.amplitude);
                self.already_set=true;
            } 
        }
        else {
            self.speed = self.speed-self.freq*value *deltatime.as_secs_f32();
        }  
        self.normal * self.speed * deltatime.as_secs_f32()
    }
    fn clone_box(&self) -> Box<dyn Movement + Send + Sync> {
        Box::new(self.clone())
    }
}