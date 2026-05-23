# Système de niveaux — Timeline déclarative

[src/level/level.rs](src/level/level.rs) pilote le déroulement du jeu via une timeline déclarative.

## Architecture : Level → Difficulty → systèmes

```
level.rs (LevelRunner)          difficulty.rs (Difficulty)         systèmes de jeu
────────────────────           ──────────────────────────         ─────────────────
SetDifficulty(3.5)     ──→     difficulty.factor = 3.5      ──→  asteroid spawn rate
StartSpawning("x",4,2) ──→     difficulty.active_spawners   ──→  green_ufo.rs, asteroid.rs
StopSpawning("x")      ──→     difficulty.active_spawners   ──→  retire le spawner
SpawnEnemy("boss",2)   ──→     difficulty.spawn_requests    ──→  boss.rs spawne 2×
StartBgDeceleration    ──→     difficulty.bg_decel_*        ──→  difficulty.rs calcule
ShowPlanet             ──→     difficulty.planet_appear_*   ──→  background.rs anime
StartMusic             ──→     commands.spawn(AudioPlayer)  ──→  music direct
PlaySound(Sfx)         ──→     SfxLibrary.get(...)          ──→  voir docs/audio.md
StartCountdown         ──→     CountdownEvent               ──→  countdown.rs
SendBoom               ──→     BoomEvent                    ──→  countdown.rs (flash)
```

Le niveau écrit dans `Difficulty` (hub central), les systèmes spécialisés lisent.

## Triggers

| Trigger | Constructeur | Description |
|---------|-------------|-------------|
| `AtTime(f32)` | `LevelStep::at(7.0, "label")` | Temps absolu depuis début niveau |
| `AfterPrevious(f32)` | `LevelStep::after(2.0, "label")` | N secondes après l'étape précédente |
| `After(&str, f32)` | `LevelStep::after_step("ref", 5.0, "label")` | N secondes après l'étape nommée |

`After` permet de chaîner à n'importe quelle étape par son label :

```rust
LevelStep::at(35.8, "boss_spawn")
    .with(Action::SpawnEnemy("boss", 1, SpawnPosition::At(0.0, 50.0))),

LevelStep::after_step("boss_spawn", 10.0, "boss1_ufos")
    .with(Action::SpawnEnemy("green_ufo", 4, SpawnPosition::Top)),
```

## Actions

| Action | Effet |
|--------|-------|
| `SetDifficulty(f32)` | Change `difficulty.factor` |
| `PlaySound(Sfx)` | SFX one-shot via la banque centralisée — voir [docs/audio.md](docs/audio.md) |
| `StartMusic(&str)` | Musique avec marker `MusicMain` |
| `StopMainMusic` | Despawn toutes les entités `MusicMain` |
| `StartCountdown` | Envoie `CountdownEvent` (READY-3-2-1-GO) |
| `SendBoom` | Envoie `BoomEvent` (flash visuel) |
| `SpawnEnemy(&str, usize, SpawnPosition)` | Spawn N ennemis d'un type (one-shot) |
| `StartSpawning(&str, usize, f32, SpawnPosition)` | Spawner continu (count/intervalle/position). Pour `"asteroid"`, count/interval ignorés (système propre basé sur `difficulty.factor`). |
| `StopSpawning(&str)` | Désactive un spawner continu |
| `SpawnSimpleUfoWave { count, interval }` | Spawn une wave de Simple UFOs en queue le long d'un chemin Bézier **random** (calculé une fois au déclenchement). Chaîner plusieurs actions de ce type à des temps différents pour avoir N waves indépendantes. Cf. [simple_ufo.rs](src/enemy/simple_ufo.rs). |
| `MarkLevelComplete` | Marque le niveau comme terminé (déclenche le countdown → outro). Émis aussi par `detect_boss_death` quand le dernier boss meurt. |
| `StartBgDeceleration { duration, final_speed }` | Décélération background |
| `ShowPlanet` | Animation d'apparition de la planète |
| `Log(&str)` | `info!()` console (debug) |

## SpawnPosition

| Variant | Description |
|---------|-------------|
| `Top` / `Bottom` / `Left` / `Right` | Hors écran sur ce bord, position aléatoire sur l'autre axe |
| `UpperMid` | Centré X, moitié haute (par défaut en éditeur) |
| `At(f32, f32)` | Position exacte (x, y) |

## LevelRunner & Difficulty

```rust
pub struct LevelRunner {
    steps: Vec<LevelStep>,
    current: usize,
    elapsed: f32,
    last_trigger_time: f32,            // pour AfterPrevious
    trigger_times: HashMap<&str, f32>, // pour After("label", delay)
}

// Dans Difficulty (hub) :
pub spawn_requests: Vec<(&str, usize, SpawnPosition)>,
pub active_spawners: HashMap<&str, (usize, f32, SpawnPosition)>,
```

**LevelActionEvent** : n'importe quel système peut injecter des actions en envoyant `LevelActionEvent(Vec<Action>)`. Utilisé par le boss pour spawner des GreenUFOs en transition, et par `detect_boss_death` pour émettre `MarkLevelComplete`.

## Gating par LevelPhase

Le `LevelRunner` et `process_level_action_events` tournent **uniquement
pendant `LevelPhase::Running`** (cf. [docs/game.md](game.md)). Pendant
l'intro et l'outro, la timeline est en pause. Aucun action ne fire et
`runner.elapsed` n'avance pas. Le gating est appliqué via
`.run_if(in_state(LevelPhase::Running))` dans [`LevelPlugin`](src/level/level.rs).

## Niveau 1 — Timeline

```
 0.0s  game_start      Music(gradius.ogg), Diff(0.5), Start(asteroid)
 7.0s  countdown       Sound(UiCountdownReady), Countdown
10.0s  phase_2_start   Diff(3.5), Start(2×green_ufo,4s), Start(mine,7s)
14.3s  boom_1          Diff(4.5), Sound(UiCountdownGo), Boom
18.3s  boom_2          Diff(6.5), Sound(UiCountdownGo), Boom
22.6s  boom_3          Diff(7.5), Sound(UiCountdownGo), Boom
27.7s  pre_boss        Stop(asteroid, green_ufo, mine), BgDecel(9s,30)
28.0s  planet_appear   Planet
35.8s  boss_spawn      Spawn(1×boss), StopMusic
```

La musique boss (`boss.ogg`) est lancée quand le premier boss atteint `Idle`, arrêtée à la mort du **dernier** boss vivant.

## Ajouter une étape

```rust
LevelStep::at(15.0, "new_event")
    .with(Action::PlaySound(crate::audio::Sfx::UiCountdownGo))
    .with(Action::SetDifficulty(5.0)),

LevelStep::after_step("boss_spawn", 10.0, "boss_rage")
    .with(Action::SetDifficulty(10.0))
    .with(Action::Log("Boss en rage !")),
```

## Ajouter un ennemi spawnable

1. `PhaseDef` + `EnemyDef` dans [enemies.rs](src/enemy/enemies.rs)
2. Module avec système de spawn qui lit `difficulty.spawn_requests` (one-shot) ou `difficulty.active_spawners` (continu)
3. Référencer dans la timeline : `Action::SpawnEnemy("nom", N, pos)` ou `Action::StartSpawning("nom", N, interval, pos)`

**Spawn one-shot** :
```rust
let Some(pos) = difficulty.spawn_requests.iter().position(|(name, _, _)| *name == "mon_ennemi") else { return; };
let (_name, count, spawn_pos) = difficulty.spawn_requests.remove(pos);
for _ in 0..count { /* spawner à spawn_pos */ }
```

**Spawner continu** :
```rust
let Some(&(wave_size, interval, spawn_pos)) = difficulty.active_spawners.get("mon_ennemi") else { return; };
// utiliser wave_size, interval, spawn_pos pour le timer
```

## Ajouter une Action

1. Variant dans `enum Action` ([level.rs](src/level/level.rs))
2. Branche dans `execute_action`
3. Cas dans `Action::short_name()` (debug overlay)
4. Si nécessaire, champ dans `Difficulty` pour la communication

## Debug — F1

Overlay timeline (panneau droite) : étapes `DONE` (temps réel), `NEXT` (temps restant), `....` (futures), liens de causalité pour `After`. F2 saute au boss, F4 win instant.
