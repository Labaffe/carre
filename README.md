# Carré — Space Shooter 2D

Jeu de type space shooter en Rust avec le moteur [Bevy 0.13](https://bevyengine.org/).
Évitez et détruisez les astéroïdes qui tombent, la difficulté augmente avec le temps.

---

## Contrôles

| Action | Touche |
|--------|--------|
| Déplacer le vaisseau | `Z` `Q` `S` `D` (AZERTY) |
| Viser | Souris |
| Tirer | Clic gauche (maintenu) |
| Mode debug | `F1` |

---

## Prérequis

- [Rust](https://www.rust-lang.org/tools/install) (edition 2024)
- Cargo (inclus avec Rust)

---

## Lancer le jeu en développement

```bash
cargo run
```

---

## Générer un `.exe` distribuable

### 1. Compiler en mode release

```bash
cargo build --release
```

L'exécutable sera généré dans :
```
target/release/carre.exe
```

### 2. Préparer le dossier de distribution

```bash
mkdir carre_jeu
cp target/release/carre.exe carre_jeu/
cp -r assets carre_jeu/
```

### 3. Structure du dossier final

```
carre_jeu/
  carre.exe
  assets/
    audio/
    fonts/
    images/
```

> Le dossier `assets/` doit obligatoirement se trouver à côté du `.exe`.
> Bevy le cherche au même niveau que l'exécutable au lancement.

### 4. Envoyer à un ami

Zippez le dossier `carre_jeu/` et envoyez-le.
Votre ami n'a besoin d'installer aucune dépendance — l'exécutable est autonome sur Windows.

---

## Structure du projet

```
src/
  main.rs         — point d'entrée, branchement des plugins
  asteroid.rs     — spawn, mouvement, HP, flash de hit
  background.rs   — fond scrollant
  collision.rs    — détection joueur asteroides
  crosshair.rs    — réticule souris
  debug.rs        — overlay debug (F1) : FPS, timer, difficulté
  difficulty.rs   — système de difficulté progressive
  explosion.rs    — animation d'explosion en 4 frames
  gameover.rs     — écran game over + restart (R)
  missile.rs      — tir, cadence, collision missile asteroides
  player.rs       — vaisseau, déplacement, rotation vers le réticule
  state.rs        — états du jeu (Playing / GameOver)
  thruster.rs     — animation du propulseur

assets/
  audio/          — musiques et effets sonores (.ogg / .wav)
  fonts/          — polices
  images/         — sprites (vaisseau, astéroïdes, missile, explosions)
```

---

## Commandes utiles

```bash
cargo run              # lancer en mode debug
cargo build            # compiler (debug)
cargo build --release  # compiler en mode optimisé (distribution)
cargo check            # vérifier sans compiler
```
