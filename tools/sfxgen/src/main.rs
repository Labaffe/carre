//! sfxgen — générateur de SFX 8-bit pour le jeu.
//!
//! Usage :
//!   cargo run -p sfxgen -- all              # génère toute la banque
//!   cargo run -p sfxgen -- one shoot        # un seul son
//!   cargo run -p sfxgen -- list             # liste les recettes disponibles
//!
//! Sortie par défaut : assets/audio/sfx/<name>.wav (relatif au workspace).

mod recipes;
mod synth;

use clap::{Parser, Subcommand};
use hound::{SampleFormat, WavSpec, WavWriter};
use std::path::{Path, PathBuf};

use crate::synth::{Sfx, SAMPLE_RATE};

#[derive(Parser)]
#[command(name = "sfxgen", about = "Générateur de SFX 8-bit")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Génère toutes les recettes dans `--out`.
    All {
        #[arg(long, default_value = "assets/audio/sfx")]
        out: PathBuf,
        /// Ne pas écraser les fichiers existants (utile si tu as déjà mis un
        /// vrai son à la place d'un généré).
        #[arg(long)]
        skip_existing: bool,
    },
    /// Génère une seule recette par nom.
    One {
        name: String,
        #[arg(long, default_value = "assets/audio/sfx")]
        out: PathBuf,
        #[arg(long)]
        skip_existing: bool,
    },
    /// Affiche les noms de recettes disponibles.
    List,
}

fn main() -> std::io::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::All { out, skip_existing } => {
            std::fs::create_dir_all(&out)?;
            for sfx in recipes::all() {
                write_sfx(&sfx, &out, skip_existing)?;
            }
        }
        Cmd::One { name, out, skip_existing } => {
            std::fs::create_dir_all(&out)?;
            let Some(sfx) = recipes::by_name(&name) else {
                eprintln!("recette inconnue : '{name}'");
                eprintln!("liste : sfxgen list");
                std::process::exit(1);
            };
            write_sfx(&sfx, &out, skip_existing)?;
        }
        Cmd::List => {
            for sfx in recipes::all() {
                println!("{}", sfx.name);
            }
        }
    }
    Ok(())
}

fn write_sfx(sfx: &Sfx, out_dir: &Path, skip_existing: bool) -> std::io::Result<()> {
    let path = out_dir.join(format!("{}.wav", sfx.name));

    if skip_existing && path.exists() {
        println!("skip (existe déjà) : {}", path.display());
        return Ok(());
    }

    let spec = WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut w = WavWriter::create(&path, spec)
        .map_err(|e| std::io::Error::other(format!("WAV create failed: {e}")))?;
    for s in sfx.render() {
        w.write_sample(s)
            .map_err(|e| std::io::Error::other(format!("WAV write failed: {e}")))?;
    }
    w.finalize()
        .map_err(|e| std::io::Error::other(format!("WAV finalize failed: {e}")))?;
    println!("écrit : {}", path.display());
    Ok(())
}
