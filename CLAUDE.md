# Carré — Jeu Bevy 2D

Jeu 2D en Rust avec le moteur [Bevy](https://bevyengine.org/) (v0.13).

## Stack technique

- **Langage** : Rust (edition 2024)
- **Moteur** : Bevy 0.13
- **Plateforme** : Windows

## Structure

```
src/main.rs   — point d'entrée unique
```

## Mécanique de jeu

Deux joueurs (sprites rectangulaires) se déplacent indépendamment :

| Joueur | Couleur | Contrôles |
|--------|---------|-----------|
| Player | Bleu    | Flèches directionnelles |
| Player2 | Rose   | ZQSD |

## Commandes

```bash
cargo run      # lancer le jeu
cargo build    # compiler
cargo test     # tests
```
