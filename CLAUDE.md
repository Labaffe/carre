# Carré — Jeu Bevy 2D

Shoot'em up vertical en Rust avec [Bevy](https://bevyengine.org/) 0.18.

## Stack

- Rust edition 2024, Bevy 0.18 (feature `wav` activée), `fastrand`
- Plateforme : Windows
- Workspace : crate principal `carre` + outil `tools/sfxgen`

## Structure

```
src/
  audio/         — banque centralisée des SFX (enum Sfx, SfxLibrary, SfxPlayer)
  behavior/     — BehaviorTree générique (ordered, choice, parallel, weighted)
  debug/        — overlay debug (F1), skips (F2/F3/F4), hitboxes, timeline
  deckbuilding/ — système de cartes (deck, hand)
  editor/       — mode test d'ennemis (lance un ennemi seul)
  enemy/        — framework ennemi + modules spécifiques (boss, mine, kamikaze, etc.)
  environment/  — background scrolling, planète
  fx/           — explosions, animations
  game_manager/ — GameState, GameProgress, intro/outro, Difficulty (hub)
  geometry/     — Shape (Circle, Rect), helpers de collision
  item/         — items (drop, ramassage), bombes
  level/        — timeline déclarative (LevelStep, Action, LevelRunner)
  menu/         — main menu, pause, level select, game over
  movement/     — Movements + stratégies (Chase, Translate, Oscilate, etc.)
  physic/       — health, collision, AOE, invulnerable, harmless, player_detection
  player/       — joueur, vies, phases ship, invincibilité
  tweening/     — animations UI
  ui/           — score, countdown, crosshair
  weapon/       — armes, projectiles, tir joueur
tools/sfxgen/   — générateur de SFX 8-bit (binaire séparé)
assets/         — images, sons, polices
docs/           — documentation détaillée (voir ci-dessous)
```

## Contrôles

| Action | Touches |
|--------|---------|
| Déplacement | ZQSD |
| Tir | Souris gauche |
| Bombe | Espace (si disponible) |
| Pause | Échap |
| Debug | F1 (overlay + timeline), F2 (skip au pre-boss), F4 (win niveau) |

## Documentation détaillée

- [docs/enemy.md](docs/enemy.md) — Framework ennemi (machine à état, boss, kamikaze, mine)
- [docs/level.md](docs/level.md) — Timeline déclarative, actions, runner, niveau 1
- [docs/game.md](docs/game.md) — GameState, GameProgress, intro/outro, mode éditeur
- [docs/items.md](docs/items.md) — Items, bombes, drop tables
- [docs/audio.md](docs/audio.md) — Banque SFX centralisée, sfxgen, conventions

## Commandes

```bash
cargo run               # lancer le jeu
cargo build             # compiler
cargo run -p sfxgen -- all   # régénérer la banque SFX 8-bit
```

## Conventions importantes

- **Despawn** : toujours `e.try_despawn()`, jamais `e.despawn()` — voir mémoire perso.
- **Sons** : passer par `SfxPlayer` + enum `Sfx`, pas de path string hardcodé. Voir [docs/audio.md](docs/audio.md).
- **Bevy SystemParam** : 16 params max par système. Bundler dans `#[derive(SystemParam)]` au besoin.
- **Animations sprites** : `Animation::new(name, duration)` boucle par défaut. Chaîner `.one_shot()` pour s'arrêter sur la dernière frame. Chaque dossier d'`AnimBank` = une animation autonome.
