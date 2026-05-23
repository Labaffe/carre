# Carré — Jeu Bevy 2D

Shoot'em up vertical en Rust avec [Bevy](https://bevyengine.org/) 0.18.

## Stack

- Rust edition 2024, Bevy 0.18 (feature `wav` activée), `fastrand`
- Plateforme : Windows
- Workspace : crate principal `carre` + outil `tools/sfxgen`

## Structure

```
src/
  audio/         — banque centralisée des SFX (enum Sfx, SfxLibrary, SfxPlayer, spawn_sfx helper)
  behavior/     — BehaviorTree générique (ordered, choice, parallel, weighted)
  debug/        — overlay debug (F1), skips (F2/F3/F4/F5), hitboxes, timeline
  deckbuilding/ — système de cartes (deck, hand)
  editor/       — mode test d'ennemis (lance un ennemi seul ou un scénario)
  enemy/        — framework ennemi + modules spécifiques :
                  asteroid, boss, mine, kamikaze, green_ufo, octopus (+ green variant),
                  turret, simple_ufo, vaisseau, enemy_group (formation parent),
                  hit_flash, anim_bank, death, enemies (config statique)
  environment/  — background scrolling, planète, nuages
  fx/           — explosions, animations
  game_manager/ — GameState, SubState LevelPhase, GameProgress, intro/outro, loading,
                  Difficulty (hub central de communication entre niveau et systèmes)
  geometry/     — Shape (Circle, Rect), helpers de collision
  item/         — items droppables (bomb, bonus_score, armor) + UI bombe/armure
  level/        — timeline déclarative (LevelStep, Action, LevelRunner, chaos)
  menu/         — main menu, pause, level select, game over
  movement/     — Movements + stratégies (Chase, Translate, Oscilate, Bezier, Goto, etc.)
  physic/       — health, collision (Hitbox/CollisionLayer/CollidesWith), AOE,
                  invulnerable, harmless, player_detection
  player/       — joueur, vies (health.png), armor (cap 3), invincibilité,
                  dash + shield (composants `EquippedPower`)
  sprite_orient/ — flip horizontal (`FaceMovement`) + rotation continue
                  vers la direction de mouvement (`RotateToMovement`)
  tweening/     — animations UI
  ui/           — score, countdown, crosshair
  weapon/       — armes, projectiles, tir joueur, UI arme
tools/sfxgen/         — générateur de SFX 8-bit (binaire séparé)
tools/sprite_hue_shift.py — script Python qui décale la teinte d'un dossier
                            de sprites (utilisé pour générer l'octopus vert)
assets/         — images, sons, polices
docs/           — documentation détaillée (voir ci-dessous)
```

## Contrôles

| Action | Touches |
|--------|---------|
| Déplacement | ZQSD |
| Tir | Souris gauche |
| Pouvoir équipé (Shield / Dash selon `EquippedPower`) | Espace |
| Bombe (consomme 1) | LShift |
| Pause | Échap |
| Debug | F1 (overlay + hitboxes + timeline), F2/F3 (skip intro), F4 (skip → outro), F5 (kill player) |

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

### Commandes ECS sûres (gotcha récurrent)

- **`e.try_despawn()`** toujours, jamais `e.despawn()`.
- **`e.try_insert(...)`** / **`e.try_remove::<C>()`** toujours, jamais `e.insert(...)` / `e.remove::<C>()`. Les non-`try_*` paniquent si l'entité a été despawn entre le `get_entity` et le flush des commands (race classique avec les bombes, le bomb_apply_damage, ou les cascades de mort). Voir le pattern dans `kamikaze_force_boom_system`, `octopus_become_alive`, `detect_death`.

### Pipeline de mort des ennemis (gotcha)

- `detect_death` ([src/enemy/death.rs](src/enemy/death.rs)) **exige** `&mut TransitionMessages` dans son query. Un ennemi SANS `BehaviorComponent` + `TransitionMessages` ne meurt jamais via ce chemin.
- Si tu fais un ennemi simple sans BT (cf. kamikaze), prévoir un système custom qui watch HP=0 (ex: `kamikaze_force_boom_system`), OU mettre un **stub BT** (cf. `turret`, `simple_ufo`) : juste un `choice` avec `alive` (composants injectés) → `dying` (transition "die") qui insère `DespawnSelf`.

### Position en monde, pas en local

- **`&GlobalTransform`** (pas `&Transform`) pour toute lecture de position destinée à de la collision, du targeting ou du rendu absolu. Les enfants d'un `EnemyGroup` (tourelles d'un vaisseau) ont un `Transform` LOCAL qui ne reflète pas le mouvement du parent. Le query collision (`detect_overlaps`) et le système de visée tourelle utilisent déjà `GlobalTransform`. Si tu écris un nouveau système qui lit la position d'une entité possiblement parentée, utilise `GlobalTransform`.

### Cleanup automatique en sortie de Playing

- Composant marker `GameplayEntity` (déclaré dans [src/main.rs](src/main.rs)) auto-cleanup à `OnExit(GameState::Playing)`. Toute entité gameplay (ennemi, AOE, UI gameplay, audio loopé) doit avoir ce marker — soit directement, soit via `#[require(crate::GameplayEntity)]` sur un composant porteur (ex: `Enemy` le require déjà).

### Patterns ECS modernes (Bevy 0.18)

- **Component hooks** (`#[component(on_insert = ...)]` / `on_remove = ...`) : utilisés pour les sons réactifs et le cleanup. Exemples : `OctopusMoving` joue `Sfx::OctopusRush` à l'insertion ; `Shielding`/`Dashing` retirent `Invulnerable` à leur retrait.
- **Observers** (`On<Event>` + `add_observer`) : utilisé pour `HitEvent` (5 observers réagissent : flash, son joueur, son ennemi, score, post-hit player). Préférer pour les events de gameplay où la réaction doit être synchrone.
- **`Single<T, F>`** au lieu de `Query<T, F>` quand "exactement 1 entité attendue" : équivalent moderne du `let Ok(x) = q.single() else { return };`. Si 0 ou 2+, le système skip silencieusement.
- **SubState** : `LevelPhase` est un `SubState<GameState::Playing>` (Intro / Running / OutroCountdown / Outro). Auto-cleanup à la sortie de Playing.

### Audio + animation

- **Sons** : passer par `SfxPlayer` + enum `Sfx`, pas de path string hardcodé. Voir [docs/audio.md](docs/audio.md). Pour les hooks de composant (`DeferredWorld`), utiliser `spawn_sfx(&mut world, Sfx::X)`.
- **Bevy SystemParam** : 16 params max par système. Bundler dans `#[derive(SystemParam)]` au besoin.
- **Animations sprites** : `Animation::new(name, duration)` boucle par défaut. Chaîner `.one_shot()` pour s'arrêter sur la dernière frame. Chaque dossier d'`AnimBank` = une animation autonome.

### Bundle helpers pour les composés

- Pour les ennemis spawnables à plusieurs endroits (standalone via builder + enfant d'un groupe), exposer un `xxx_bundle(asset_server, position) -> impl Bundle` qui retourne tous les composants. Voir [src/enemy/turret.rs](src/enemy/turret.rs) (`turret_bundle`, `turret_module_bundle`) et [src/enemy/simple_ufo.rs](src/enemy/simple_ufo.rs) (`simple_ufo_bundle` + `SimpleUfoWaveSpawner`).

### EnemyGroup (formations)

- Pattern pour les ennemis composés (vaisseau à tourelles, etc.) :
  - Parent = entité avec `EnemyGroup` + `DespawnWhenChildrenEmpty` + `Movements` + `Transform` + `Visibility::default()`.
  - Enfants = ennemis standards spawn via `with_children`. Bevy propage `GlobalTransform`.
  - Le système `despawn_empty_groups` despawn le parent quand aucun enfant n'a plus `Enemy` → cascade.
  - Voir [src/enemy/vaisseau.rs](src/enemy/vaisseau.rs).

## Modules fondateurs — NE PAS MODIFIER sans demander

Certains modules constituent les **briques de base** du projet, écrites par l'équipe (notamment l'ami contributeur). Tu peux les *utiliser* librement, mais tu ne dois **PAS modifier, refactorer, étendre ou ajouter des fonctionnalités** dans ces modules sans poser une question claire à l'utilisateur d'abord et obtenir un OK explicite.

Cette règle s'applique à :

- [src/deckbuilding/](src/deckbuilding/) — système de cartes (card_hand, card_played, card_deck, layout, etc.)
- [src/environment/](src/environment/) **uniquement la partie clouds** — système de nuages procéduraux
- [src/tweening/](src/tweening/) — animations UI (tween, plugin, component)
- [src/movement/](src/movement/) — moteur de déplacement (Movements, Chase, Translate, Oscilate, Spin, Goto, etc.)
- [src/behavior/](src/behavior/) — BehaviorTree générique (ordered_list, choice_list, parallel_node_list, component_container, etc.)

**Concrètement** :
- ✅ Tu peux *appeler* `Chase::new(speed)`, `BehaviorBuilder::choice()`, `Tween::new(...)`, etc. depuis du code consommateur (ennemis, items, UI…)
- ✅ Tu peux *lire* leur code pour comprendre comment ils marchent
- ❌ Tu ne dois pas modifier leur signature, ajouter un champ, changer leur comportement, ajouter un nouveau type de mouvement/behavior/tween, refactorer leur API
- ❌ Si tu penses qu'il faut une nouvelle primitive (ex: un nouveau type de `Movement`), **arrête-toi et demande** avant d'écrire la moindre ligne

Si un besoin réel apparaît (un cas légitime nécessite une primitive manquante), formule la question : "Pour faire X, j'aurais besoin d'ajouter Y dans `src/movement/` — est-ce qu'on touche à ce module ou je trouve un autre chemin ?". L'utilisateur arbitrera.
