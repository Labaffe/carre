# Carré — Jeu Bevy 2D

Shoot'em up vertical en Rust avec le moteur [Bevy](https://bevyengine.org/) (v0.13).

## Stack technique

- **Langage** : Rust (edition 2024)
- **Moteur** : Bevy 0.13
- **Plateforme** : Windows

## Structure

```
src/
  main.rs        — point d'entrée, enregistrement des plugins
  state.rs       — GameState (MainMenu, Playing, GameOver)
  player.rs      — joueur, vies, invincibilité
  missile.rs     — tir et collision missiles/astéroïdes
  weapon.rs      — définition des hitboxes (Circle, Rect)
  asteroid.rs    — astéroïdes (spawn, mouvement, rotation)
  enemy.rs       — framework générique ennemi (composants, systèmes réutilisables)
  enemies.rs     — registre central de tous les ennemis (EnemyDef, PhaseDef, PatternDef)
  boss.rs        — boss : systèmes spécifiques (intro, flexing, charge, musique)
  collision.rs   — collision joueur ↔ entités hostiles (trait Hittable)
  explosion.rs   — animations d'explosion
  difficulty.rs  — difficulté progressive, timers de paliers
  countdown.rs   — countdown "3-2-1-GO" avec animations pop
  background.rs  — scrolling de fond
  mainmenu.rs    — menu principal avec tiles aléatoirement invisibles
  gameover.rs    — écran game over
  pause.rs       — pause (Echap), run condition not_paused()
  crosshair.rs   — viseur
  debug.rs       — overlay debug (F1), skip (F2/F3), hitboxes
```

## Contrôles

| Action | Touches |
|--------|---------|
| Déplacement | Flèches directionnelles |
| Tir | Automatique |
| Pause | Echap |
| Debug | F1 (overlay), F2 (skip astéroïdes), F3 (skip au boss) |

## Framework ennemi — Machine à état

Le framework `enemy.rs` fournit une machine à état générique pour tous les ennemis :

```
Entering ──→ Flexing ──→ Idle ──→ Active(0) ──→ Active(1) ──→ … ──→ Dying ──→ Dead
```

| État | Description | Vulnérable | Dangereux |
|------|-------------|------------|-----------|
| `Entering` | Animation d'arrivée (optionnelle) | Non | Non |
| `Flexing` | Animation post-arrivée (optionnelle) | Non | Non |
| `Idle` | Attente avant le combat (optionnelle), animation idle, immobile | Non | Non |
| `Active(n)` | Phase de combat n, patterns actifs | Oui | Oui |
| `Dying` | Animation de mort (shake, flash, explosions) | Non | Non |
| `Dead` | Entité despawnée | — | — |

**Phases** : chaque ennemi définit une liste de `PhaseDef` dans `enemies.rs`. Une phase a ses propres PV et sa liste de patterns. Quand les PV tombent à 0, passage à la phase suivante ou mort si dernière phase.

**Patterns** : chaque `PatternDef` a un nom et une durée. Le pattern executor cycle à travers la liste et reprogramme le timer avec la durée du prochain pattern. Un pattern peut aussi se terminer par un événement (ex: la charge du boss se termine au contact du mur).

**Systèmes génériques** (dans `EnemyPlugin`) :
- Dégâts missile → ennemi (uniquement en `Active`)
- Flash blanc au hit
- Transition de phase (0 PV → phase suivante ou mort)
- Animation de mort (shake + explosions)
- Déplacement des projectiles ennemis
- Mouvement patrol sinusoïdal

**Ajouter un nouvel ennemi** :
1. Définir ses `PhaseDef` et son `EnemyDef` dans `enemies.rs`
2. Créer un module avec ses systèmes spécifiques (intro, patterns)
3. Les systèmes génériques fonctionnent automatiquement

Un ennemi simple peut spawner directement en `Active(0)` sans passer par `Entering`/`Flexing`.

## Commandes

```bash
cargo run      # lancer le jeu
cargo build    # compiler
cargo test     # tests
```
