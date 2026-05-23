# Framework ennemi

Tous les ennemis vivent dans [src/enemy/](src/enemy/). Pas de machine à
état fermée — chaque ennemi compose les briques disponibles selon sa
complexité.

## Briques disponibles

- **`Enemy::new(EnemyData)`** : marker + nom (debug/scoring). Requires
  `GameplayEntity` (auto-cleanup à la sortie de Playing).
- **`Health::new(N)`** : PV courants/max + helpers `take_damage`, `heal`,
  `is_dead`. Couplé à `apply_damage` (pipeline central — cf. `physic/health.rs`).
- **`collider(shape, layer, mask)`** : retourne `(Hitbox, CollisionLayer, CollidesWith)`.
  Layers définies dans `physic/collider.rs` (`ENEMY`, `PLAYER`, `ASTEROID`,
  `PLAYER_PROJECTILE`, `ENEMY_PROJECTILE`, `AOE`, `ITEM`).
- **`Movements::new().with(Movement)`** : pile additive de mouvements.
  Stratégies dans `src/movement/` : `Chase`, `Goto`, `Bezier`, `Oscilate`,
  `Translate`, `Spin`, `Rush`, etc.
- **`Animation::new(name, frame_dur)`** : animation préchargée dans
  `AnimBank` (par dossier d'assets). `.one_shot()` pour s'arrêter sur la
  dernière frame.
- **`BehaviorComponent`** + **`TransitionMessages`** : BT générique
  (`src/behavior/`) pour les ennemis multi-états. Voir plus bas.
- **`DropTable { drops: &'static [(ItemType, f32)] }`** : items lâchés à
  la mort (cf. [docs/items.md](items.md)).
- **`FaceMovement`** : flip horizontal du sprite selon le sens de mouvement.
- **`RotateToMovement`** : rotation **continue** du sprite vers la tangente
  du mouvement (vs `FaceMovement` qui ne fait qu'un flip). Utilisé par
  `simple_ufo`.
- **`DespawnOffScreen`** : auto-despawn quand l'entité quitte l'écran avec
  marge (`movement/despawn_off_screen.rs`).
- **`PlayerDetection`** : trigger sur entrée/sortie d'une zone autour du joueur.

## Pipeline de dégâts (centralisé)

```
Émetteurs ──→ DamageEvent ──→ apply_damage ──→ HitEvent (trigger) ──→ Observers
(projectile,    (target,        (filtre           (target,                ↓
 player coll,    amount,         Invulnerable      target_layer,      hit_flash_on_hit
 bomb...)        source)         + Invincible      amount_dealt,      enemy_hit_sound_on_hit
                                 + take_damage,    source)            score_on_enemy_hit
                                 consomme Armor                       player_post_hit
                                 si présente)                         play_player_hurt_sfx
```

**Détails** :
- `HitEvent` n'est **plus** un Message — c'est un `Event` trigger via
  `commands.trigger(HitEvent { ... })`. 5 observers globaux y réagissent.
- `apply_damage` consomme d'abord les points d'`Armor` (player) si présent
  avant de toucher `Health`. Voir [docs/items.md](items.md).
- Pour qu'un ennemi meure proprement (drops + cleanup), il lui faut
  `BehaviorComponent` + `TransitionMessages` : `detect_death` les exige dans
  son query, écrit `DropEvent`, pousse `"die"` dans les messages, insère
  le marker `Dying`. Le BT transitionne alors vers son état `dying`
  (typiquement → `DespawnSelf` → despawn en PostUpdate).

### Sans BehaviorTree

Pour un ennemi simple sans BT (cf. **kamikaze**), `detect_death` ne fire
jamais. Deux alternatives :
1. **Système custom** qui watch HP=0 et insère directement les markers de
   mort + écrit `DropEvent`. Exemple : `kamikaze_force_boom_system` +
   `kamikaze_boom_system` qui spawn l'AOE et écrit le DropEvent.
2. **BT stub** : un `BehaviorBuilder::choice()` avec un état alive
   (composants injectés) → transition `"die"` → état dying qui insère
   `DespawnSelf`. Exemples : **turret**, **simple_ufo**.

## Roster

| Ennemi | Comportement | Notes |
|--------|-------------|-------|
| **Asteroid** | Chute verticale, hit-flash, DropTable. Pas de BT — un système dédié gère la mort + spawn de FX. | Le plus simple. |
| **GreenUFO** | BT : choice(idle, rush). Patrouille horizontale + rush rectiligne. Anim death one-shot puis DespawnSelf. | |
| **Mine** | BT : falling → counting_down → exploding (`MineExplode` hook joue le son). AOE statique. `PlayerDetection` déclenche le countdown. | `BlinkRed` pendant countdown. |
| **Kamikaze** | **Pas de BT** depuis la simplif. Chase + `KamikazeSpeedRamp` (s'accélère avec l'âge). Boom (AOE + drop + despawn) sur HP=0 OU contact joueur via `kamikaze_force_boom_system`. Rire en loop attaché en child à la naissance (cascade auto à la mort). | Sprite statique (`frame003`), pas d'animation. |
| **Octopus** | BT complexe : entering_rush → alive (choice : telegraph → swoop OU shoot → repeat) → dying. Swoop = Bézier vers miroir-joueur + jitter. Shoot = 3 projectiles roses en éventail. Hooks `on_insert` sur markers de phase pour les sons. | Pré-swoop = 1 son d'avertissement. |
| **OctopusGreen** | Variante : marker `OctopusGreen` posé EN PLUS de `Octopus`. Systèmes filtrés via `Without<OctopusGreen>` / `With<OctopusGreen>`. Swoop = cible **aléatoire** + **intangible** (collider retiré, sprite assombri). Tir = 4 projectiles verts plus larges. Largue des **bombes vertes en cloche** pendant le swoop (`OctopusGreenBomb` + `OctopusGreenBombThrower`) qui explosent en AOE à l'impact. Sprites générés par hue-shift (`tools/sprite_hue_shift.py`). | |
| **Turret** | Statique, sprite gatling animé en boucle. Vise le joueur via `GlobalTransform` (fonctionne en child d'un groupe). Tir périodique. Module séparé (sprite `turret_module.png`) en entité-frère qui reste après la mort. | Stub BT pour l'éclair de mort. |
| **SimpleUfo** | Pas d'IA. Suit un chemin Bézier (start → mid → end, durée paramétrée). `RotateToMovement` aligne le sprite sur la tangente. Stub BT pour mort + drops. Spawn via `SimpleUfoWaveSpawner` pour effet "queue leu leu". | 4 HP, 20 frames d'anim. |
| **Vaisseau** | Premier exemple d'`EnemyGroup` : parent invisible avec `Movements` (Oscilate X + Y → Lissajous) + 4 tourelles enfants. Despawn auto quand toutes les tourelles sont mortes via `DespawnWhenChildrenEmpty`. Module sprite par tourelle (entité-frère, persiste après mort de la gatling). | |
| **Boss** | Multi-phase (3 phases × 100 PV). Spirale intro, transitions avec `Invulnerable` + spawn de GreenUFOs. Anim death longue (shake, flash, explosions). | Voir [boss.rs](src/enemy/boss.rs). |

## EnemyGroup — Formations composites

Pattern pour les ennemis multi-composants (vaisseau à tourelles, ver, etc.) :

- **Parent** : entité avec :
  - `EnemyGroup` (marker informatif)
  - `DespawnWhenChildrenEmpty` (despawn quand aucun enfant n'a plus `Enemy`)
  - `Transform` + `Visibility::default()` (requis sans Sprite pour que les enfants soient rendus)
  - `Movements` (la formation entière bouge ici)
- **Enfants** : ennemis standards (avec `Enemy`, `Health`, collider, etc.)
  spawnés via `with_children`. Bevy propage `GlobalTransform`.

```rust
commands.spawn((
    EnemyGroup,
    DespawnWhenChildrenEmpty,
    Transform::from_xyz(x, y, 0.5),
    Visibility::default(),
    Movements::new().with(Oscilate::new(...)),
))
.with_children(|p| {
    p.spawn(turret_bundle(asset_server, Vec3::new(offset_x, 0., 0.)));
});
```

**Système** : [`despawn_empty_groups`](src/enemy/enemy_group.rs) check à
chaque frame si un parent a encore des enfants `Enemy` vivants. Sinon, le
parent despawn → cascade Bevy nettoie les enfants résiduels (sprites,
sons d'explosion en cours, etc.).

⚠️ **Position en monde** : les systèmes qui lisent la position d'un
enfant pour de la collision ou du targeting doivent utiliser
`&GlobalTransform`, pas `&Transform`. Sinon ils voient l'offset local et
manquent le déplacement du parent. Le système collision (`detect_overlaps`)
et `turret_aim_and_fire` sont déjà conformes.

## Wave spawner (queue leu leu)

Pour spawn N ennemis successifs avec un comportement partagé (ex: tous
les UFOs d'une wave suivent la même trajectoire random) :

1. **Composant contrôleur** (ex: [`SimpleUfoWaveSpawner`](src/enemy/simple_ufo.rs)) :
   stocke les paramètres communs (path, count restant, intervalle) + un
   `Timer` Repeating.
2. **Système** ([`simple_ufo_wave_spawn_system`](src/enemy/simple_ufo.rs)) :
   tick le timer, spawn 1 ennemi à chaque `just_finished`, décrémente,
   auto-despawn quand `remaining == 0`.
3. **Action level** ([`Action::SpawnSimpleUfoWave`](src/level/level.rs)) :
   spawn juste l'entité contrôleur, qui s'initialise lazy au 1er tick
   (besoin de `Single<&Window>` pour le random path → indisponible dans
   `apply_action`).

Pour faire 3 vagues random différentes : chaîner 3 `Action::SpawnSimpleUfoWave`
à des temps différents dans la timeline.

## Patterns audio

- **Hooks `on_insert`** pour les sons réactifs au changement d'état :
  `Sfx::OctopusRush` quand `OctopusMoving` est inséré, `Sfx::Explosion`
  quand `KamikazeBoom` est inséré, etc. Utilise le helper
  `spawn_sfx(&mut world, Sfx::X)` (cf. `audio/mod.rs`).
- **Observer global** sur `HitEvent` pour les sons de hit
  (`play_player_hurt_sfx`, `enemy_hit_sound_on_hit`).
- **Audio en child** pour les boucles attachées à un ennemi
  (`kamikaze_laugh_start_system` spawn un `AudioPlayer::new(...).LOOP` en
  child du kamikaze → cascade despawn auto à la mort).

## Bundle helpers (pour réutilisation parent/child)

Pour un ennemi qui peut être spawn standalone OU comme enfant d'un groupe,
exposer une fonction `xxx_bundle(asset_server, position) -> impl Bundle`
qui retourne tous les composants. Exemples :

- [`turret_bundle`](src/enemy/turret.rs) + [`turret_module_bundle`](src/enemy/turret.rs)
- [`simple_ufo_bundle`](src/enemy/simple_ufo.rs)

Le `EnemyBuilder::spawn` standalone appelle alors juste
`commands.spawn(xxx_bundle(asset_server, pos.extend(0.5)))`.

## Ajouter un nouvel ennemi

1. **Const stats** dans [`enemies.rs`](src/enemy/enemies.rs) :
   ```rust
   pub const MY_ENEMY: EnemyData = EnemyData {
       name: "MyEnemy",
       config: EnemyConfigData::new(radius, sprite_size),
       total_hp: N,
   };
   ```
2. **Nouveau module** `src/enemy/my_enemy.rs` :
   - `MyEnemyBuilder` (impl `EnemyBuilder` avec `name()`, `preload_anim()`,
     `spawn()`)
   - Components custom si besoin (timers, markers de phase…)
   - Systèmes spécifiques (visée, tir, etc.)
3. **Register** dans [`src/enemy/mod.rs`](src/enemy/mod.rs) :
   - `pub mod my_enemy;`
   - Import du builder + systèmes
   - `.with(MyEnemyBuilder::new())` dans `EnemyRegister`
   - Ajout des systèmes dans le tuple `Update`
4. **Mort** :
   - Soit BT avec un `dying` state qui insère `DespawnSelf` (laisse
     `detect_death` gérer drops + transition)
   - Soit un système custom watch HP=0 (cf. kamikaze)
5. **Éditeur** : ajouter une ligne dans `EDITOR_ENEMIES`
   ([src/menu/mainmenu.rs](src/menu/mainmenu.rs)).
