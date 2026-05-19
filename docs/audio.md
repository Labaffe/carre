# Système audio

[src/audio/mod.rs](src/audio/mod.rs) — un seul point de vérité pour les SFX. La musique garde son système séparé (markers).

## Principe

- L'enum `Sfx` liste **tous** les events sonores du jeu, par sémantique
- Deux variantes peuvent pointer vers le même fichier (`Sfx::UiCountdownBeep` et `Sfx::MineBeep` partagent `t_1.ogg`) : les variantes représentent les **events**, pas les fichiers
- `SFX_PATHS` (const array dans [audio/mod.rs](src/audio/mod.rs)) est la **seule** place où vivent les chemins. Swap `.wav` ↔ `.ogg` = changer une ligne.
- Préchargement au démarrage : `SfxLibrary` (Resource) tient un `Handle<AudioSource>` par variante → zéro hitch au premier play

## API : `SfxPlayer` SystemParam

```rust
fn my_system(mut sfx: SfxPlayer) {
    sfx.play(Sfx::PlayerShoot);                  // DESPAWN auto
    sfx.play_at(Sfx::PlayerHurt, 3.0);           // volume custom
    sfx.play_once(Sfx::PlayerDeath);             // PlaybackSettings::ONCE (pas d'auto-despawn)
}
```

Retourne `EntityCommands` pour chaîner un marker si nécessaire :

```rust
sfx.play(Sfx::ShipArrival).insert(IntroSound);   // marker pour pause sync
```

## Conventions sémantiques

Un fichier physique peut être réutilisé, mais chaque event sonore distinct a sa propre variante d'enum. Cela permet de diverger plus tard sans toucher les call sites.

Exemples de splits actuels :

| Fichier | Variantes (events) |
|---------|---------------------|
| `t_1.ogg` | `UiCountdownBeep` (3-2-1) + `MineBeep` (amorce mine) |
| `bomb.ogg` | `PlayerBomb` (espace) + `MineExplode` (boom mine) |
| `level_up.ogg` | `ItemAppear` (drop) + `ScoreMilestone` (palier score) |
| `landing.ogg` | `PlanetLanding` (planète) + `ShipArrival` (intro vaisseau) |

Pour ajouter un nouvel event sonore (même si le fichier existe déjà pour un autre event) : ajouter une variante dans `enum Sfx` + une ligne dans `SFX_PATHS`.

## Action::PlaySound dans la timeline

Action de level utilise une variante Sfx, pas un path string :

```rust
LevelStep::at(7.0, "countdown")
    .with(Action::PlaySound(crate::audio::Sfx::UiCountdownReady))
    .with(Action::StartCountdown),
```

## Musique — système distinct

Les musiques (`audio/music/*.ogg`) NE passent PAS par `SfxLibrary`. Elles utilisent leur propre marker pour cleanup et pause sync :

- `MusicMain` — musique de niveau
- `MusicBoss` — musique de boss
- `MusicOutro` — `stage_clear.ogg`
- `MusicGameOver` — game over
- `IntroSound` — marqueur sur le SFX d'intro (`ShipArrival`) pour synchroniser la pause

Lancement : `Action::StartMusic("audio/music/foo.ogg")`.

## EnemyConfigData — hit/die sounds par ennemi

Chaque `EnemyData` dans [enemies.rs](src/enemy/enemies.rs) déclare ses propres `hit_sound` et `death_explosion_sound` (path strings). Ces sons ne passent **pas** par l'enum `Sfx` — ils sont déjà isolés per-enemy par design. Pour donner un son spécifique au boss : changer le champ de `BOSS` dans `enemies.rs`.

## Outil sfxgen — Génération de SFX 8-bit

[tools/sfxgen/](tools/sfxgen/) — petit crate du workspace qui génère des `.wav` 8-bit (square, triangle, noise, sweep, ADSR, bit-crush, lowpass) à partir de recettes Rust.

```bash
cargo run -p sfxgen -- list
cargo run -p sfxgen -- all                 # régénère toute la banque (overwrite)
cargo run -p sfxgen -- all --skip-existing # préserve les vrais sons posés à la main
cargo run -p sfxgen -- one shoot           # une seule recette
```

**Recettes** dans [tools/sfxgen/src/recipes.rs](tools/sfxgen/src/recipes.rs). Chaque recette = une fonction Rust qui retourne un `Sfx` (le builder de sfxgen, à ne pas confondre avec l'enum `Sfx` du jeu) :

```rust
pub fn shoot() -> Sfx {
    Sfx::new("shoot")
        .wave(Wave::Square)
        .freq_sweep(1200.0, 600.0)
        .duration(0.08)
        .envelope(0.0, 0.005, 1.0, 0.07)
        .volume(0.35)
}
```

Output : `assets/audio/sfx/<name>.wav` (22050 Hz, 16-bit mono).

**Remplacer un son généré par un vrai** : enregistrer le fichier sous le même nom (`.wav` ou `.ogg` + ajuster `SFX_PATHS`), utiliser `--skip-existing` pour ne plus l'écraser à la régénération.

## Volume global

`GameSettings.master_volume` (0.0–1.0, défaut 0.3) appliqué via `GlobalVolume` au démarrage. Pas (encore) de séparation music/SFX au niveau du mixer.

## Ajouter un son

1. **Si nouveau fichier** : déposer dans `assets/audio/sfx/` (ou générer via sfxgen)
2. Ajouter variante dans `enum Sfx` ([audio/mod.rs](src/audio/mod.rs))
3. Ajouter ligne dans `SFX_PATHS`
4. Au call site : `sfx.play(Sfx::MaVariante)`

## Limite Bevy SystemParam — 16 params

Les systèmes Bevy 0.18 ont une limite de 16 params dans le tuple. `SfxPlayer` compte comme **1**. Si un système approche la limite, bundler des params dans une struct `#[derive(SystemParam)]` (exemple : `ConfirmParams` dans [pause.rs](src/menu/pause.rs)).
