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
  asteroid.rs    — astéroïdes (spawn, mouvement, rotation, drop table)
  enemy.rs       — framework générique ennemi (composants, systèmes réutilisables)
  enemies.rs     — registre central de tous les ennemis (EnemyDef, PhaseDef, PatternDef)
  boss.rs        — boss : systèmes spécifiques (intro, flexing, transition, charge, musique)
  green_ufo.rs   — GreenUFO : ennemi simple (rush/idle, mort instantanée)
  collision.rs   — collision joueur ↔ entités hostiles (trait Hittable)
  explosion.rs   — animations d'explosion
  item.rs        — items (drop, animation, ramassage), bombes, bonus score
  difficulty.rs  — difficulté progressive, timers de paliers
  countdown.rs   — countdown "3-2-1-GO" avec animations pop
  background.rs  — scrolling de fond
  mainmenu.rs    — menu principal avec tiles aléatoirement invisibles
  gameover.rs    — écran game over
  pause.rs       — pause (Echap), run condition not_paused()
  crosshair.rs   — viseur
  score.rs       — score du joueur, UI
  debug.rs       — overlay debug (F1), skip (F2/F3), hitboxes
```

## Contrôles

| Action | Touches |
|--------|---------|
| Déplacement | ZQSD |
| Tir | Automatique |
| Bombe | Espace (si disponible) |
| Pause | Echap |
| Debug | F1 (overlay), F2 (skip astéroïdes), F3 (skip au boss) |

## Framework ennemi — Machine à état

Le framework `enemy.rs` fournit une machine à état générique pour tous les ennemis :

```
Entering ──→ Flexing ──→ Idle ──→ Active(0) ──┬──→ Transitioning(1) ──→ Active(1) ──→ …
                                               └──→ Dying ──→ Dead
```

| État | Description | Vulnérable | Dangereux |
|------|-------------|------------|-----------|
| `Entering` | Animation d'arrivée (optionnelle) | Non | Non |
| `Flexing` | Animation post-arrivée (optionnelle) | Non | Non |
| `Idle` | Attente avant le combat (optionnelle), animation idle, immobile | Non | Non |
| `Active(n)` | Phase de combat n, patterns actifs | Oui | Oui |
| `Transitioning(n)` | Animation de transition vers la phase n (shake + flash, pas d'explosions). Géré par le module spécifique de l'ennemi. | Non | Non |
| `Dying` | Animation de mort (shake, flash, explosions) | Non | Non |
| `Dead` | Entité despawnée | — | — |

**Phases** : chaque ennemi définit une liste de `PhaseDef` dans `enemies.rs`. Une phase a ses propres PV, sa liste de patterns, et un flag `has_transition`. Quand les PV tombent à 0 :
- Si `has_transition = true` → entre en `Transitioning(next)`, l'animation est gérée par le module spécifique de l'ennemi
- Si `has_transition = false` → passage direct à `Active(next)` ou `Dying` si dernière phase

**Patterns** : chaque `PatternDef` a un nom et une durée. Le pattern executor cycle à travers la liste et reprogramme le timer avec la durée du prochain pattern. Un pattern peut aussi se terminer par un événement (ex: la charge du boss se termine au contact du mur).

**Systèmes génériques** (dans `EnemyPlugin`) :
- Dégâts missile → ennemi (uniquement en `Active`)
- Flash blanc au hit
- Transition de phase (0 PV → phase suivante, transition ou mort)
- Animation de mort (shake + explosions)
- Déplacement des projectiles ennemis
- Mouvement patrol sinusoïdal

**Ajouter un nouvel ennemi** :
1. Définir ses `PhaseDef` et son `EnemyDef` dans `enemies.rs`
2. Créer un module avec ses systèmes spécifiques (intro, patterns)
3. Si `has_transition = true`, implémenter les systèmes de transition dans le module spécifique
4. Les systèmes génériques fonctionnent automatiquement

Un ennemi simple peut spawner directement en `Active(0)` sans passer par `Entering`/`Flexing`.

**La machine à état est entièrement optionnelle.** Un ennemi peut utiliser tous les états ou seulement une partie :

| Ennemi | États utilisés | Mort |
|--------|---------------|------|
| **Boss** | `Entering` → `Flexing` → `Idle` → `Active(0)` → `Transitioning(1)` → `Active(1)` → … → `Dying` → `Dead` | Longue (4s, shake + explosions + flexing accéléré) |
| **GreenUFO** | `Active(0)` → `Dying` → `Dead` | Instantanée (explosion style astéroïde) |

## Système d'items

Le fichier `item.rs` gère les items droppés par les entités mortes. Architecture event-driven :

1. Une entité avec `DropTable` meurt → `DropEvent` émis
2. `process_drop_events` spawne les items (sprite animé, descente)
3. Le joueur touche un item → effet appliqué + son + despawn

**Types d'items :**

| Item | Effet | Animation | Son apparition | Son ramassage |
|------|-------|-----------|----------------|---------------|
| `Bomb` | +1 bombe au compteur | `images/bomb/` | `level_up.ogg` | `level_up.ogg` |
| `BonusScore` | +50 au score | `images/bonus_score/` | `level_up.ogg` | `level_up.ogg` |

**Bombes** : le joueur accumule des bombes (UI sous les vies). Appuyer sur Espace consomme 1 bombe :
- Flash blanc plein écran (fade out 0.4s)
- Son `bomb.ogg`
- Dégâts à tous les astéroïdes (`BOMB_DAMAGE_ASTEROID = 999`) et ennemis actifs (`BOMB_DAMAGE_ENEMY = 50`)
- Texte `[ESPACE]` clignote sous les icônes de bombes quand le compteur > 0

**Drop tables** :
- Astéroïdes : `[(Bomb, 5%), (BonusScore, 10%)]`
- GreenUFO : `[(Bomb, 10%), (BonusScore, 15%)]`

Chaque entrée est testée indépendamment.

## Commandes

```bash
cargo run      # lancer le jeu
cargo build    # compiler
cargo test     # tests
```
