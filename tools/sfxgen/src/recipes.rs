//! Banque de recettes SFX. Chaque fonction retourne un `Sfx` complet.
//! Ajouter une recette = ajouter une fonction + une entrée dans `all()`.

use crate::synth::{Sfx, Wave};

pub fn shoot() -> Sfx {
    Sfx::new("shoot")
        .wave(Wave::Square)
        .freq_sweep(1200.0, 600.0)
        .duration(0.08)
        .envelope(0.0, 0.005, 1.0, 0.07)
        .volume(0.35)
}

pub fn hit() -> Sfx {
    Sfx::new("hit")
        .wave(Wave::Noise)
        .freq_sweep(800.0, 200.0)
        .duration(0.06)
        .envelope(0.0, 0.001, 0.3, 0.05)
        .volume(0.45)
}

pub fn explode() -> Sfx {
    Sfx::new("explode")
        .wave(Wave::Noise)
        .freq_sweep(400.0, 60.0)
        .duration(0.55)
        .envelope(0.005, 0.05, 0.4, 0.5)
        .lowpass(1800.0)
        .volume(0.55)
}

pub fn pickup() -> Sfx {
    Sfx::new("pickup")
        .wave(Wave::Triangle)
        .freq_arpeggio(&[523.25, 783.99, 1046.50])
        .duration(0.18)
        .envelope(0.0, 0.02, 0.7, 0.15)
        .volume(0.4)
}

pub fn kamikaze_warn() -> Sfx {
    Sfx::new("kamikaze_warn")
        .wave(Wave::Square)
        .freq_sweep(440.0, 880.0)
        .duration(0.3)
        .envelope(0.0, 0.05, 0.6, 0.2)
        .bit_crush(6)
        .volume(0.4)
}

pub fn all() -> Vec<Sfx> {
    vec![shoot(), hit(), explode(), pickup(), kamikaze_warn()]
}

pub fn by_name(name: &str) -> Option<Sfx> {
    match name {
        "shoot" => Some(shoot()),
        "hit" => Some(hit()),
        "explode" => Some(explode()),
        "pickup" => Some(pickup()),
        "kamikaze_warn" => Some(kamikaze_warn()),
        _ => None,
    }
}
