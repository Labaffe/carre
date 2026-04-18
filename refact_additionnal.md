Je vais analyser le code en profondeur pour proposer une architecture plus scalable.

# Analyse architecturale — vers un éditeur de niveau

## 1. État actuel : forces et points de friction

### Points forts (à conserver)
- **`EnemyDef` / `PhaseDef` / `PatternDef`** : le data-driven est déjà amorcé pour les ennemis (`enemies.rs`).
- **`WeaponDef`** : armes joueur déjà déclaratives.
- **`Difficulty` comme hub** : single source of truth bien identifié.
- **`level.rs` timeline déclarative** : `LevelStep::at(...).with(Action::...)` est l'embryon d'un éditeur.
- **Machine à état générique `EnemyState`** : couvre 90 % des comportements (Entering → Active → Dying).
- **Système d'items event-driven** (`DropEvent`).

### Frictions empêchant la scalabilité

| # | Problème | Conséquence pour un éditeur |
|---|----------|----------------------------|
| 1 | Patterns identifiés par `&'static str` (`"rush"`, `"patrol"`, `"charge"`) avec `match` dispersés dans chaque module | Ajouter un pattern = toucher 3 modules, aucune complétion IDE, aucun typage |
| 2 | Spawn requests typées par `&'static str` (`"green_ufo"`, `"boss"`) | Couplage faible, pas de validation au compile |
| 3 | Comportement de chaque ennemi dispersé : `boss.rs` (1500+ lignes), `green_ufo.rs`, `gatling.rs` ont chacun leurs 8-12 systèmes | Pas de réutilisation, doublons (rush, charge, patrol existent en N variantes) |
| 4 | `Difficulty` est devenue un sac à tout : 20+ champs (`boss_music_played`, `phase3_charging_played`, `landing_played`, etc.) | Couplage fort entre modules, race conditions (`boss_seen_alive`), difficile à raisonner |
| 5 | Pas d'abstraction sur les projectiles : `Missile` (joueur) vs `EnemyProjectile` (ennemi) sont deux mondes parallèles avec doublons | Impossible de partager les behaviors (homing, spread, multi-hit) |
| 6 | Niveaux sont du **code Rust** (`build_level_1()`) | Pas d'édition à chaud, pas d'éditeur visuel possible |
| 7 | Animations frames chargées de 3 manières (`GatlingFrames`, `GreenUFOFrames`, `BossIdleFrames`…) | N résources presque identiques |
| 8 | Sons inlinés dans le code (`commands.spawn(AudioBundle{...})`) | Pas de mixage centralisé, volume/canal géré ad hoc |

---

## 2. Architecture cible — trois couches

```
┌──────────────────────────────────────────────────────────────┐
│  COUCHE DONNÉES (sérialisable)                              │
│  Level / Wave / EnemyTemplate / WeaponTemplate / Pattern    │
│  (RON/JSON, éditables sans recompil)                        │
└──────────────────────────────────────────────────────────────┘
                          ↓ (instanciation)
┌──────────────────────────────────────────────────────────────┐
│  COUCHE COMPOSANTS (Bevy ECS)                               │
│  Movement, Shooter, Targeting, Health, Hitbox, AnimSet…    │
│  Composants atomiques composables                           │
└──────────────────────────────────────────────────────────────┘
                          ↓ (systèmes génériques)
┌──────────────────────────────────────────────────────────────┐
│  COUCHE SYSTÈMES                                            │
│  Un système par composant, jamais "par ennemi"              │
└──────────────────────────────────────────────────────────────┘
```

L'idée : **plus aucun fichier `boss.rs` / `green_ufo.rs`**. Un boss = une config `EnemyTemplate` + une `Wave` qui le spawne. Le boss n'est pas un *type* spécial, c'est juste un `Enemy` avec plus de phases et de composants.

---

## 3. Traits clés à introduire

### 3.1 `MovementBehavior` — déplacements composables

```rust
pub trait MovementBehavior: Send + Sync + 'static {
    fn update(&mut self, ctx: &MovementCtx, transform: &mut Transform, dt: f32) -> MovementResult;
}

pub struct MovementCtx<'a> {
    pub player_pos: Vec2,
    pub window: Vec2,
    pub elapsed: f32,
}

pub enum MovementResult { Continue, Done }
```

Implémentations stockées comme **composants** :
- `LinearMove { velocity }`
- `Patrol { speed_x, sine_amp, sine_freq }` ← déjà existant
- `Charge { target: ChargeTarget, speed }`
- `Rush { direction }` ← unifie `GreenUFORush` et `BossCharge`
- `Homing { speed, turn_rate }`
- `Spiral { center, angular_speed, radius_growth }` ← l'intro du boss
- `BezierPath { points, duration }`

Un seul système `apply_movement<T: MovementBehavior>` itère sur tous les composants implémentant le trait.

### 3.2 `ShootPattern` — tirs composables

```rust
pub trait ShootPattern: Send + Sync + 'static {
    fn shots(&self, ctx: &ShootCtx) -> Vec<ProjectileSpawn>;
}

pub struct ProjectileSpawn {
    pub origin: Vec2,
    pub direction: Vec2,
    pub template: ProjectileTemplate,
}
```

Implémentations :
- `SingleShot { aim: AimMode }`
- `Spread { count, total_angle, aim }`
- `Aimed { lead_factor }` ← aim_and_shoot
- `Sweep { half_cone, frequency, phase }` ← le balayage gatling
- `Burst { count, interval }`
- `RingShot { count }`

Un composant `Shooter { pattern: Box<dyn ShootPattern>, fire_timer, projectile: ProjectileTemplate }`.
Un seul système `shooter_system` qui tire pour tout le monde (ennemi ou joueur).

### 3.3 `Targeting` — ciblage abstrait

```rust
pub enum AimMode {
    Forward,                            // direction du sprite
    AtPlayer,                           // verrouille le joueur
    AtPlayerWithCone { half_cone: f32 },
    AtPosition(Vec2),
    Sweep { half_cone, freq, phase, bias },
}
```

→ unifie tout le code de visée éparpillé dans `gatling.rs` et `boss.rs`.

### 3.4 `ProjectileTemplate` — le projectile unifié

```rust
pub struct ProjectileTemplate {
    pub sprite: SpriteSpec,
    pub hitbox: HitboxShape,           // déjà existant
    pub speed: f32,
    pub damage: i32,
    pub team: Team,                    // Player | Enemy
    pub behavior: ProjectileBehavior,  // Straight | Homing { target } | Sine { amp, freq }
    pub lifetime: Option<f32>,
    pub on_hit: Vec<HitEffect>,        // Damage, Pierce, Explode { radius }, Split
    pub death_anim: Option<AnimSet>,
    pub trail: Option<TrailSpec>,
}
```

→ supprime `Missile`, `EnemyProjectile`, `BossProjectile` etc. Un seul `Projectile` component, distingué par `team`.

### 3.5 `EnemyTemplate` — bundle complet

```rust
pub struct EnemyTemplate {
    pub name: String,
    pub sprite: SpriteSpec,
    pub anim_set: AnimSet,             // idle, hit, death, intro
    pub hitbox: HitboxShape,
    pub phases: Vec<PhaseTemplate>,
    pub intro: Option<IntroSpec>,      // spirale, top-drop, instant
    pub death: DeathSpec,              // shake, explosion, instant
    pub drops: Vec<DropEntry>,
    pub sounds: SoundSet,
}

pub struct PhaseTemplate {
    pub health: i32,
    pub movement: Vec<MovementSpec>,   // séquence de mouvements (rush 0.4s → idle 0.2s)
    pub shooting: Vec<ShootSpec>,      // patterns de tir parallèles
    pub transition: Option<TransitionSpec>,
}
```

À l'instanciation, `EnemyTemplate::spawn()` insère les bons composants (`Patrol`, `Shooter`, `Charge`, `AnimSet`, etc.).

### 3.6 `Hittable` (existe déjà) → étendre en trait

Tu as déjà `Hittable` dans `collision.rs`. Le généraliser :

```rust
pub trait Hittable {
    fn hitbox(&self) -> Hitbox;
    fn team(&self) -> Team;
    fn on_hit(&mut self, damage: i32) -> HitOutcome;
}
```

→ collisions joueur/ennemi/projectile/item passent toutes par le même système.

---

## 4. Refonte des niveaux — vers l'éditeur

### 4.1 Format de données

Passer de `build_level_1()` (Rust) à un fichier RON :

```ron
Level(
    name: "Space Invader",
    intro: ( duration: 3.0, sound: "audio/intro/lvl1.ogg" ),
    music: "audio/music/gradius.ogg",
    bg_speed: 150.0,
    timeline: [
        Step( label: "phase_2", at: AtTime(7.0), actions: [
            SetDifficulty(3.5),
            StartSpawning( enemy: "green_ufo", count: 4, interval: 4.0, from: Top ),
        ]),
        Step( label: "boss_spawn", at: AtTime(35.8), actions: [
            SpawnEnemy( template: "boss_v1", count: 1, at: Position(0, 50) ),
            StopMusic,
        ]),
        Step( label: "boss_ufos", at: AfterStep("boss_spawn", 10.0), actions: [
            SpawnEnemy( template: "green_ufo", count: 4, at: Top ),
        ]),
    ],
)
```

Chargé via `bevy_asset_loader` + `serde`. Hot-reload natif Bevy permettrait de modifier le niveau pendant que le jeu tourne.

### 4.2 Templates également sérialisés

`assets/enemies/boss.ron`, `assets/enemies/green_ufo.ron`, `assets/weapons/red_projectile.ron`. Le code Rust ne contient plus *aucune* donnée de gameplay.

### 4.3 Éditeur in-game

Avec `bevy_egui` :
- Panneau "Timeline" : drag-and-drop des `Step`, sliders pour `AtTime`
- Panneau "Enemy editor" : choisir movement, shoot pattern, visualisation live
- Panneau "Test" : F5 = spawn ce qu'il y a sous le curseur, immédiatement
- Sauvegarde RON automatique

---

## 5. Découpage en modules cible

```
src/
  core/
    state.rs           — GameState
    team.rs            — Team enum (Player|Enemy|Neutral)
    hitbox.rs          — HitboxShape (déjà), Hittable
  components/          — composants atomiques réutilisables
    movement.rs        — Patrol, Charge, Rush, Homing, Spiral, Bezier
    shooter.rs         — Shooter, AimMode, fire_timer
    health.rs          — Health, Phases, transitions
    anim.rs            — AnimSet, AnimController
    drop.rs            — DropTable
  templates/           — data layer
    enemy.rs           — EnemyTemplate
    weapon.rs          — WeaponTemplate
    projectile.rs      — ProjectileTemplate
    level.rs           — LevelTemplate (RON)
  systems/             — un système = un composant
    movement_systems.rs
    shooter_systems.rs
    projectile_systems.rs
    collision_systems.rs
    death_systems.rs
  level/
    runner.rs          — LevelRunner (déjà), évalue le timeline
    actions.rs         — Action enum (déjà), exécution
    loader.rs          — RON loader
  player/              — spécifique
  ui/                  — menus, HUD, pause, gameover
  editor/              — bevy_egui (optionnel, feature flag)
```

**Règle de découpage** : un module = une responsabilité. Pas de `boss.rs` car le boss n'est pas une responsabilité — c'est une *donnée*.

---

## 6. Plan de migration progressif (sans tout casser)

| Étape | Effort | Bénéfice immédiat |
|-------|--------|-------------------|
| **1. Unifier `Movement`** : créer composants `Patrol`, `Rush`, `Charge`, `Spiral`. Migrer `green_ufo` puis `boss` | M | Suppression de ~30 % de `boss.rs` et `green_ufo.rs` |
| **2. Unifier `Projectile`** : `Missile` + `EnemyProjectile` → `Projectile { team }`. Adapter collisions. | M | Pattern de tir partagé joueur/ennemi |
| **3. Composant `Shooter` + `ShootPattern`** | M | `gatling.rs` divisé par 3, ajout d'un nouveau pattern = 1 struct |
| **4. Découper `Difficulty`** en plusieurs ressources : `LevelTime`, `SpawnQueue`, `BgState`, `BossEvents` | S | Lisibilité, fin des race conditions |
| **5. `EnemyTemplate` + spawn générique** | L | Plus besoin d'un module par ennemi |
| **6. Sérialisation RON** des templates et niveaux | M | Hot reload, base pour l'éditeur |
| **7. Éditeur `bevy_egui`** | L | Productivité × 10 pour créer du contenu |

Chaque étape est indépendante — tu peux t'arrêter à 4 et déjà avoir un code beaucoup plus sain.

---

## 7. Recommandations transversales

1. **Bannir les `&'static str` comme identifiants de gameplay**. Utiliser des `enum` ou des `Handle<EnemyTemplate>`.
2. **Ne plus jamais écrire un système `xxx_specific_thing`**. Si tu en as besoin, c'est qu'il manque un composant atomique.
3. **Events plutôt que polling sur `Difficulty`**. Au lieu de `boss_music_played: bool`, émettre `BossActiveEvent` une fois.
4. **`AnimSet` unifié** avec un loader unique : `AnimSet { idle: Vec<Handle>, hit: Vec<Handle>, death: Vec<Handle> }`. Une seule `Resource<AnimRegistry>` au lieu de N.
5. **Un seul `SoundRegistry`** centralisé avec mixage (volume par catégorie : sfx/music/voice).
6. **Tests unitaires** sur les composants atomiques (un `Patrol` se teste sans écran).
7. **Feature flag `editor`** : l'éditeur ne pèse pas en build release.

---

## 8. Vision "éditeur" — minimum viable

Avec les traits ci-dessus en place, un éditeur basique (1-2 jours de dev) permet :

```
[+] Add enemy        Boss (boss_v1.ron)         [Edit]
[+] Add wave         Wave 1 @ t=10s, 4×green_ufo  [Edit]

Timeline ━━━━━●━━━━━━●━━━●━━━━━━━━━━━━━━━●━━━━━━
              ↑       ↑   ↑                ↑
           start   wave1 boom1           boss

[Edit Enemy: boss_v1]
  Sprite      [▼ assets/sprites/mothership_2.png]
  Phase 1
    Health    [████████ 100]
    Movement  [▼ Patrol]   speed: 200   amp: 0.85
    Shooter   [▼ Spread]   count: 5     angle: 30°
    Transition [✓] shake
  [+ Add phase]
```

C'est exactement le même `EnemyTemplate` qui sera sérialisé/désérialisé.

---

## TL;DR

Le code actuel est **bien conçu pour 2 ennemis**, mais chaque nouvel ennemi/pattern coûte un module entier. Le verrou architectural à lever : **arrêter de penser "type d'ennemi" et commencer à penser "composition de behaviors"**. Une fois les traits `MovementBehavior` / `ShootPattern` / `EnemyTemplate` en place, ajouter un boss = écrire un fichier RON, et un éditeur visuel devient possible avec `bevy_egui`.

L'investissement migratoire vaut largement la peine si tu prévois >5 ennemis ou >3 niveaux.