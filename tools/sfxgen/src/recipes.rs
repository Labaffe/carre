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

/// Cri d'éveil de l'octopus — bref motif descendant en triangle, grave et
/// "organique". Joué à l'apparition et au wind-up du shoot. Lowpass + bit
/// crush pour un grain rétro étouffé "sous-marin".
pub fn octopus_sound() -> Sfx {
    Sfx::new("octopus_sound")
        .wave(Wave::Triangle)
        .freq_sweep(520.0, 180.0)
        .duration(0.28)
        .envelope(0.01, 0.05, 0.7, 0.18)
        .lowpass(1400.0)
        .bit_crush(5)
        .volume(0.45)
}

/// Tir des 3 projectiles : sawtooth qui descend vite, bit-crush prononcé
/// pour un côté "blob" plutôt qu'un laser propre. Court et claquant.
pub fn octopus_shoot() -> Sfx {
    Sfx::new("octopus_shoot")
        .wave(Wave::Sawtooth)
        .freq_sweep(680.0, 220.0)
        .duration(0.14)
        .envelope(0.0, 0.01, 0.5, 0.12)
        .lowpass(2200.0)
        .bit_crush(5)
        .volume(0.4)
}

/// Whoosh de dash — noise filtré passe-bas, attaque rapide puis release
/// pour un effet de "passage rapide". Joué au début de chaque courbe Bézier.
pub fn octopus_rush() -> Sfx {
    Sfx::new("octopus_rush")
        .wave(Wave::Noise)
        .freq(1.0) // peu importe pour Noise
        .duration(0.35)
        .envelope(0.015, 0.08, 0.65, 0.25)
        .lowpass(900.0)
        .volume(0.35)
}

pub fn all() -> Vec<Sfx> {
    vec![
        shoot(),
        hit(),
        explode(),
        pickup(),
        kamikaze_scream(),
        kamikaze_laugh(),
        octopus_sound(),
        octopus_shoot(),
        octopus_rush(),
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
        "octopus_sound" => Some(octopus_sound()),
        "octopus_shoot" => Some(octopus_shoot()),
        "octopus_rush" => Some(octopus_rush()),
        _ => None,
    }
}
