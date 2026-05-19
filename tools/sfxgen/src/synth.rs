//! Primitives audio pour la génération de SFX 8-bit.
//!
//! Tout est mono, 22050 Hz, 16-bit signé. Format intentionnellement low-fi
//! pour un grain rétro et des fichiers minuscules.

use std::f32::consts::TAU;

pub const SAMPLE_RATE: u32 = 22050;

#[derive(Clone, Copy)]
pub enum Wave {
    Square,
    Triangle,
    Sawtooth,
    Noise,
}

#[derive(Clone, Copy)]
pub struct Envelope {
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
}

impl Envelope {
    /// Évalue l'enveloppe à l'instant `t` (en secondes) sur une note de durée
    /// totale `dur`. Modèle ADSR standard : ramp up sur `attack`, descente
    /// vers le niveau `sustain` sur `decay`, hold, puis ramp down sur
    /// `release`. Si `attack + decay + release > dur`, on tronque.
    pub fn eval(&self, t: f32, dur: f32) -> f32 {
        let release_start = (dur - self.release).max(0.0);
        if t < self.attack {
            t / self.attack.max(1e-6)
        } else if t < self.attack + self.decay {
            let k = (t - self.attack) / self.decay.max(1e-6);
            1.0 + (self.sustain - 1.0) * k
        } else if t < release_start {
            self.sustain
        } else {
            let k = (t - release_start) / self.release.max(1e-6);
            (self.sustain * (1.0 - k)).max(0.0)
        }
    }
}

/// Recette de son : tous les paramètres typés, immuable une fois construite.
pub struct Sfx {
    pub name: String,
    wave: Wave,
    freq_start: f32,
    freq_end: f32,
    arpeggio: Option<Vec<f32>>,
    duration: f32,
    envelope: Envelope,
    volume: f32,
    lowpass: Option<f32>,
    bit_crush: Option<u32>,
    noise_seed: u32,
}

impl Sfx {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            wave: Wave::Square,
            freq_start: 440.0,
            freq_end: 440.0,
            arpeggio: None,
            duration: 0.1,
            envelope: Envelope { attack: 0.0, decay: 0.01, sustain: 1.0, release: 0.05 },
            volume: 0.4,
            lowpass: None,
            bit_crush: None,
            noise_seed: 0x1234_5678,
        }
    }

    pub fn wave(mut self, w: Wave) -> Self { self.wave = w; self }
    pub fn duration(mut self, d: f32) -> Self { self.duration = d; self }
    pub fn volume(mut self, v: f32) -> Self { self.volume = v; self }
    pub fn lowpass(mut self, cutoff_hz: f32) -> Self { self.lowpass = Some(cutoff_hz); self }
    pub fn bit_crush(mut self, bits: u32) -> Self { self.bit_crush = Some(bits); self }
    pub fn freq(mut self, hz: f32) -> Self { self.freq_start = hz; self.freq_end = hz; self }
    pub fn freq_sweep(mut self, start: f32, end: f32) -> Self {
        self.freq_start = start; self.freq_end = end; self
    }
    pub fn freq_arpeggio(mut self, notes: &[f32]) -> Self {
        self.arpeggio = Some(notes.to_vec()); self
    }
    pub fn envelope(mut self, a: f32, d: f32, s: f32, r: f32) -> Self {
        self.envelope = Envelope { attack: a, decay: d, sustain: s, release: r };
        self
    }
    pub fn seed(mut self, seed: u32) -> Self { self.noise_seed = seed; self }

    /// Fréquence à l'instant `t` : interp linéaire sweep ou note d'arpège
    /// (notes égales partagées sur `duration`).
    fn current_freq(&self, t: f32) -> f32 {
        if let Some(notes) = &self.arpeggio {
            let step = self.duration / notes.len() as f32;
            let idx = ((t / step) as usize).min(notes.len() - 1);
            notes[idx]
        } else {
            let k = (t / self.duration).clamp(0.0, 1.0);
            self.freq_start + (self.freq_end - self.freq_start) * k
        }
    }

    /// Rend la recette en samples i16 prêts à écrire en WAV.
    pub fn render(&self) -> Vec<i16> {
        let n = (self.duration * SAMPLE_RATE as f32) as usize;
        let mut out: Vec<f32> = Vec::with_capacity(n);
        let mut phase: f32 = 0.0;
        let mut rng = self.noise_seed.max(1);

        for i in 0..n {
            let t = i as f32 / SAMPLE_RATE as f32;
            let freq = self.current_freq(t);
            phase += freq / SAMPLE_RATE as f32;
            phase -= phase.floor();

            let raw = match self.wave {
                Wave::Square   => if phase < 0.5 { 1.0 } else { -1.0 },
                Wave::Triangle => 1.0 - 4.0 * (phase - 0.5).abs(),
                Wave::Sawtooth => 2.0 * phase - 1.0,
                Wave::Noise    => {
                    // xorshift32, déterministe via `noise_seed`
                    rng ^= rng << 13;
                    rng ^= rng >> 17;
                    rng ^= rng << 5;
                    (rng as f32 / u32::MAX as f32) * 2.0 - 1.0
                }
            };

            let env = self.envelope.eval(t, self.duration);
            let mut s = raw * env * self.volume;
            if let Some(bits) = self.bit_crush { s = bitcrush(s, bits); }
            out.push(s);
        }

        if let Some(cutoff) = self.lowpass {
            lowpass_inplace(&mut out, cutoff);
        }

        out.into_iter()
            .map(|s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
            .collect()
    }
}

/// Quantification sur `bits` niveaux → grain rétro typique des consoles 8-bit.
fn bitcrush(s: f32, bits: u32) -> f32 {
    let levels = (1 << bits) as f32;
    (s * levels).round() / levels
}

/// IIR 1-pôle passe-bas appliqué in-place. `cutoff` en Hz.
fn lowpass_inplace(samples: &mut [f32], cutoff: f32) {
    let dt = 1.0 / SAMPLE_RATE as f32;
    let rc = 1.0 / (TAU * cutoff);
    let alpha = dt / (rc + dt);
    let mut prev = 0.0;
    for s in samples.iter_mut() {
        prev += alpha * (*s - prev);
        *s = prev;
    }
}
