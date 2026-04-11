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
  level.rs       — timeline déclarative du niveau (LevelStep, Trigger, Action, LevelRunner)
  difficulty.rs  — ressource Difficulty (hub de communication level → systèmes de jeu)
  countdown.rs   — countdown "3-2-1-GO" avec animations pop
  background.rs  — scrolling de fond, planète
  mainmenu.rs    — menu principal avec tiles aléatoirement invisibles
  gameover.rs    — écran game over
  pause.rs       — pause (Echap), run condition not_paused()
  crosshair.rs   — viseur
  score.rs       — score du joueur, UI
  debug.rs       — overlay debug (F1), skip (F2/F3), hitboxes, timeline niveau
```

## Contrôles

| Action | Touches |
|--------|---------|
| Déplacement | ZQSD |
| Tir | Automatique |
| Bombe | Espace (si disponible) |
| Pause | Echap |
| Debug | F1 (overlay + timeline), F2 (skip astéroïdes), F3 (skip au boss) |

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

## Système de niveaux — Timeline déclarative

Le fichier `level.rs` pilote le déroulement du jeu via une timeline déclarative. Il remplace les dizaines de booléens et timers qui étaient éparpillés dans `difficulty.rs`.

### Architecture : Level → Difficulty → systèmes

```
level.rs (LevelRunner)          difficulty.rs (Difficulty)         systèmes de jeu
────────────────────           ──────────────────────────         ─────────────────
SetDifficulty(3.5)     ──→     difficulty.factor = 3.5      ──→  asteroid spawn rate
StopAsteroidSpawning   ──→     difficulty.spawning_stopped  ──→  asteroid.rs arrête
StartSpawning("x",2)   ──→     difficulty.active_spawners   ──→  green_ufo.rs active
SpawnEnemy("boss")     ──→     difficulty.spawn_requests    ──→  boss.rs spawne
StartBgDeceleration    ──→     difficulty.bg_decel_*        ──→  difficulty.rs calcule
ShowPlanet             ──→     difficulty.planet_appear_*   ──→  background.rs anime
StartMusic / PlaySound ──→     commands.spawn(AudioBundle)  ──→  audio direct
StartCountdown         ──→     CountdownEvent               ──→  countdown.rs
SendBoom               ──→     BoomEvent                    ──→  countdown.rs (flash)
```

Le système de niveau écrit dans `Difficulty` (hub central), et les systèmes spécialisés lisent ces valeurs.

### Triggers — 3 modes de déclenchement

| Trigger | Constructeur | Description |
|---------|-------------|-------------|
| `AtTime(f32)` | `LevelStep::at(7.0, "label")` | Temps absolu depuis le début du niveau |
| `AfterPrevious(f32)` | `LevelStep::after(2.0, "label")` | N secondes après l'étape précédente |
| `After(&str, f32)` | `LevelStep::after_step("ref", 5.0, "label")` | N secondes après l'étape nommée |

Le trigger `After` permet de chaîner des événements à n'importe quelle étape par son label, pas uniquement la précédente. Exemple :

```rust
LevelStep::at(35.8, "boss_spawn")
    .with(Action::SpawnBoss),

// 5s après "boss_spawn", peu importe les étapes entre les deux
LevelStep::after_step("boss_spawn", 5.0, "boss_music")
    .with(Action::StartMusic("audio/boss.ogg")),
```

### Actions disponibles

| Action | Effet |
|--------|-------|
| `SetDifficulty(f32)` | Change `difficulty.factor` (vitesse astéroïdes, spawn rate) |
| `PlaySound(&str)` | Son one-shot (despawn auto) |
| `StartMusic(&str)` | Musique avec composant `MusicMain` |
| `StopMainMusic` | Despawn toutes les entités `MusicMain` |
| `StartCountdown` | Envoie `CountdownEvent` (READY-3-2-1-GO) |
| `SendBoom` | Envoie `BoomEvent` (flash visuel) |
| `SpawnEnemy(&str)` | Spawn one-shot via `spawn_requests` (ex: `"boss"`) |
| `StartSpawning(&str, f32)` | Spawner continu via `active_spawners` (ex: `"green_ufo"`, 2.0) |
| `StopSpawning(&str)` | Désactive un spawner continu |
| `StopAsteroidSpawning` | `difficulty.spawning_stopped = true` |
| `StartBgDeceleration { duration, final_speed }` | Décélération progressive du background |
| `ShowPlanet` | Déclenche l'animation d'apparition de la planète |
| `Log(&str)` | `info!()` en console (debug uniquement) |

### LevelRunner — état du déroulement

```rust
pub struct LevelRunner {
    steps: Vec<LevelStep>,           // étapes du niveau
    current: usize,                   // prochaine étape à exécuter
    elapsed: f32,                     // temps écoulé
    last_trigger_time: f32,           // pour AfterPrevious
    trigger_times: HashMap<&str, f32> // pour After("label", delay)
}

// Dans Difficulty (hub de communication) :
pub spawn_requests: HashSet<&str>,       // spawns one-shot (consommés)
pub active_spawners: HashMap<&str, f32>, // spawners continus (nom → intervalle)
```

Le runner parcourt les étapes dans l'ordre. Quand le déclencheur d'une étape est atteint, toutes ses actions s'exécutent et le temps est enregistré dans `trigger_times` pour les `After`.

### Niveau 1 — Timeline

```
 0.0s  game_start      Music(gradius.ogg), Diff(0.5)
 7.0s  countdown       Sound(charging.ogg), Countdown
10.0s  phase_2_start   Diff(3.5), Start(green_ufo,2s)
14.3s  boom_1          Diff(4.5), Sound(t_go.wav), Boom
18.3s  boom_2          Diff(6.5), Sound(t_go.wav), Boom
22.6s  boom_3          Diff(7.5), Sound(t_go.wav), Boom
27.7s  pre_boss        StopAst, Stop(green_ufo), BgDecel(9s,30)
28.0s  planet_appear   Planet
35.8s  boss_spawn      Spawn(boss), StopMusic
      boss_spawn_2    Spawn(boss)  [+30s -> boss_spawn]
```

Chaque boss gère sa propre séquence interne (Entering → Flexing → Idle → Active) car elle dépend de l'état du boss, pas du temps absolu. La musique boss (`boss.ogg`) est lancée une seule fois quand le premier boss atteint Idle, et ne s'arrête qu'à la mort du **dernier** boss vivant.

### Ajouter une étape au niveau

```rust
// Dans build_level_1()
LevelStep::at(15.0, "new_event")
    .with(Action::PlaySound("audio/alert.ogg"))
    .with(Action::SetDifficulty(5.0)),

// Ou chaîné à un événement existant
LevelStep::after_step("boss_spawn", 10.0, "boss_rage")
    .with(Action::SetDifficulty(10.0))
    .with(Action::Log("Boss en rage !")),
```

### Ajouter un nouvel ennemi spawnable

1. Définir ses `PhaseDef` et `EnemyDef` dans `enemies.rs`
2. Créer son module avec un système de spawn qui lit `difficulty.spawn_requests` (one-shot) ou `difficulty.active_spawners` (continu)
3. Ajouter le spawn dans la timeline : `Action::SpawnEnemy("nom")` ou `Action::StartSpawning("nom", interval)`

Exemple pour un spawner continu :
```rust
// Dans le système de spawn du nouvel ennemi :
let Some(&interval) = difficulty.active_spawners.get("mon_ennemi") else { return; };
// ... utiliser interval pour le timer de spawn
```

### Ajouter une nouvelle Action

1. Ajouter le variant dans `enum Action` (level.rs)
2. Implémenter dans `execute_action` (level.rs)
3. Ajouter un cas dans `Action::short_name()` pour le debug
4. Si nécessaire, ajouter un champ dans `Difficulty` pour la communication

### Debug : timeline dans l'overlay (F1)

L'overlay debug affiche un panneau à droite avec la timeline du niveau :
- `DONE` : étape exécutée (avec le temps réel)
- `NEXT` : prochaine étape (avec le temps restant)
- `....` : étapes futures
- Liens de causalité affichés pour les triggers `After`

## Commandes

```bash
cargo run      # lancer le jeu
cargo build    # compiler
cargo test     # tests
```
