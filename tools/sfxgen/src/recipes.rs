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

/// Cri terrifiant joué à l'entrée de la phase armed du kamikaze.
/// Sweep aigu→grave bruité + bit-crush sévère pour un côté "agonie".
pub fn kamikaze_scream() -> Sfx {
    Sfx::new("kamikaze_scream")
        .wave(Wave::Noise)
        .freq_sweep(2400.0, 200.0)
        .duration(1.2)
        .envelope(0.01, 0.08, 0.7, 0.4)
        .bit_crush(4)
        .volume(0.55)
}

/// Rire en continu pendant la phase pursuing — court motif (~0.7s) bouclé
/// par `PlaybackSettings::LOOP`. Arpège square bit-crushé + lowpass pour un
/// "ha ha ha" grave et menaçant.
pub fn kamikaze_laugh() -> Sfx {
    Sfx::new("kamikaze_laugh")
        .wave(Wave::Square)
        .freq_arpeggio(&[
            130.0, 195.0, 80.0, 130.0, 195.0, 260.0, 130.0, 80.0,
        ])
        .duration(0.7)
        .envelope(0.005, 0.02, 0.55, 0.04)
        .lowpass(600.0)
        .bit_crush(5)
        .volume(0.4)
}

pub fn all() -> Vec<Sfx> {
    vec![
        shoot(),
        hit(),
        explode(),
        pickup(),
        kamikaze_scream(),
        kamikaze_laugh(),
    ]
}

pub fn by_name(name: &str) -> Option<Sfx> {
    match name {
        "shoot" => Some(shoot()),
        "hit" => Some(hit()),
        "explode" => Some(explode()),
        "pickup" => Some(pickup()),
        "kamikaze_scream" => Some(kamikaze_scream()),
        "kamikaze_laugh" => Some(kamikaze_laugh()),
        _ => None,
    }
}
