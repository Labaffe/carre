//! Banque de recettes SFX. Chaque fonction retourne un `Sfx` complet.
//! Ajouter une recette = ajouter une fonction + une entrée dans `all()`.

use crate::synth::{Sfx, Wave};

/// "Pew" doux pour le tir joueur. Triangle (moins d'harmoniques que Square),
/// freq basse, lowpass agressif → pas de fatigue auditive même à cadence
/// rapide (RED_PROJECTILE = 6-7 shots/s).
pub fn shoot() -> Sfx {
    Sfx::new("shoot")
        .wave(Wave::Triangle)
        .freq_sweep(520.0, 220.0)
        .duration(0.08)
        .envelope(0.0, 0.005, 0.8, 0.07)
        .lowpass(1600.0)
        .volume(0.22)
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

/// Whoosh supersonic du dash joueur — sweep aigu→médium très rapide,
/// sawtooth pour le grain "déchirure d'air", lowpass mid-high pour garder
/// du tranchant sans être strident. Court (~0.18s = durée du dash).
pub fn player_dash() -> Sfx {
    Sfx::new("player_dash")
        .wave(Wave::Sawtooth)
        .freq_sweep(2600.0, 500.0)
        .duration(0.18)
        .envelope(0.0, 0.02, 0.6, 0.12)
        .lowpass(2800.0)
        .bit_crush(6)
        .volume(0.35)
}

/// Fanfare 8-bit joyeuse pour le level-up : arpège ascendant en do majeur
/// sur 2 octaves (C-E-G-C-E-G). Square wave pour le grain rétro classique,
/// attaque sèche et release moyenne pour une fanfare claire.
pub fn level_up() -> Sfx {
    Sfx::new("level_up")
        .wave(Wave::Square)
        .freq_arpeggio(&[
            523.25,  // C5
            659.25,  // E5
            783.99,  // G5
            1046.50, // C6
            1318.51, // E6
            1567.98, // G6
        ])
        .duration(0.55)
        .envelope(0.0, 0.02, 0.8, 0.18)
        .volume(0.45)
}

/// Cri de mort — sweep descendant grave, triangle bit-crushé filtré lowpass
/// pour un "gloup" agonisant. Volume soutenu, dure ~0.6s pour couvrir
/// l'animation de mort.
pub fn octopus_die() -> Sfx {
    Sfx::new("octopus_die")
        .wave(Wave::Triangle)
        .freq_sweep(420.0, 70.0)
        .duration(0.6)
        .envelope(0.0, 0.06, 0.6, 0.45)
        .lowpass(800.0)
        .bit_crush(4)
        .volume(0.55)
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
        octopus_die(),
        player_dash(),
        level_up(),
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
        "octopus_die" => Some(octopus_die()),
        "player_dash" => Some(player_dash()),
        "level_up" => Some(level_up()),
        _ => None,
    }
}
