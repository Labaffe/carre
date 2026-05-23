# Game manager — Progression et machine à état

[src/game_manager/](src/game_manager/) gère la machine à état globale, la
sous-phase d'un niveau, la progression campagne/primes et la séquence d'outro.

## GameState

```
MainMenu → LevelSelect → Loading → Playing → LevelTransition → LevelSelect → …
              ↓                       ↓
            (Editor)                GameOver → MainMenu
                                      ↓
                                    Credits → MainMenu
```

| État | Description |
|------|-------------|
| `MainMenu` | Menu principal (Commencer, Niveaux, Éditeur, Paramètres, Quitter) |
| `LevelSelect` | Écran de sélection de niveaux (Campagne ou Primes) |
| `Loading` | Écran de chargement court (~0.5s) avant `Playing` — masque les hitchs initiaux (asset decode, glyph rasterization, musique). Routé par `LevelSelect` et `GameOver` restart. |
| `Playing` | Niveau en cours. Voir `LevelPhase` ci-dessous. |
| `LevelTransition` | État transitoire entre deux niveaux (déclenche cleanup puis setup). |
| `Editor` | Mode éditeur — sélection d'un ennemi à tester. |
| `Credits` | Écran "Merci d'avoir joué" en fin de campagne. |
| `GameOver` | Écran de game over. |

Défini dans [src/game_manager/state.rs](src/game_manager/state.rs).

## LevelPhase — SubState de Playing

Sous-état dans `Playing`, géré comme un **SubState Bevy** (auto-créé à
l'entrée de `Playing`, auto-supprimé à la sortie). Plus de `Resource` qui
traîne. Définit dans [src/game_manager/game.rs](src/game_manager/game.rs).

```rust
#[derive(SubStates, ...)]
#[source(GameState = GameState::Playing)]
pub enum LevelPhase {
    #[default] Intro,
    Running,
    OutroCountdown,
    Outro,
}
```

| Phase | Effet | Données éphémères |
|-------|-------|-------------------|
| `Intro` | Animation d'arrivée du vaisseau + son `ShipArrival`. Freeze gameplay via `pause.intro_active=true`. Termine quand son ET animation finis. **Skippée en mode éditeur**. | `IntroData` (timer, positions, ratio) |
| `Running` | LevelRunner exécute les étapes (cf. [docs/level.md](level.md)). | (aucune) |
| `OutroCountdown` | 3s d'attente après `MarkLevelComplete`. | `OutroCountdownData` (timer) |
| `Outro` | Freeze + musique `stage_clear.ogg` + texte victoire. Entrée/Espace → niveau suivant ou MainMenu/Credits. | `OutroData` (elapsed, music_spawned) |

### Pattern : Resources éphémères

Chaque phase qui a besoin de données mutables expose une Resource (`IntroData`,
`OutroCountdownData`, `OutroData`) insérée par le `OnEnter(LevelPhase::X)`
handler et retirée par `OnExit(LevelPhase::X)`. Les systèmes d'une phase
font `Res<XxxData>` (ou `Option<Res<...>>` quand le mode éditeur peut
court-circuiter — cas de `IntroData` qui est absente en éditeur).

### Gating des systèmes

```rust
.add_systems(
    Update,
    (intro_tick, skip_intro_input).run_if(in_state(LevelPhase::Intro)),
)
```

Le `LevelRunner` lui-même tourne sous `run_if(in_state(LevelPhase::Running))`
(cf. [src/level/level.rs](src/level/level.rs)).

## GameProgress + PlayMode

```rust
pub struct GameProgress {
    pub current_level: usize,  // 1-indexed
    pub total_levels: usize,
}

pub enum PlayMode { Campaign, Primes }
```

- **Campagne** : finir tous les niveaux en séquence. Progression persistée
  dans `CampaignProgress.completed: HashSet<usize>`. Quitter en cours
  efface tout (popup de confirmation `ConfirmPopup`).
- **Primes** : jouer un niveau à la carte depuis le sélecteur.

`setup_level` lit `current_level` et choisit le builder approprié
(`build_level_1()`, `build_level_2()`, `build_level_chaos()`, etc.).

## Outro — Séquence de victoire

```
MarkLevelComplete (boss mort OU action timeline)
  ↓
OutroCountdown (3s)
  ↓
Outro (freeze + musique stage_clear + UI)
  ↓
Entrée/Espace → LevelSelect (campagne ou primes)
              ou Credits (dernier niveau campagne)
              ou MainMenu (mode non défini)
```

- **`enter_outro` (OnEnter)** : `pause.outro_active=true`, despawn musiques
  gameplay, stoppe le background, insère `OutroData`, spawn UI.
- **Musique** : `stage_clear.ogg` une seule fois (flag `music_spawned`).
- **Input** : après `OUTRO_INPUT_DELAY` (3s), Entrée/Espace continue.

## Mode éditeur

Ressource [`EditorTestEnemy(name)`](src/level/level.rs) insérée par le
menu éditeur. Quand présente au moment de `setup_level` :

- Timeline minimale dépendante du nom : pour la plupart 1 ennemi à
  `SpawnPosition::UpperMid`. **Special-case `simple_ufo`** : 4 vagues de
  6 UFOs en queue le long de chemins random (démontre le pattern wave).
- Phase `Intro` **skippée** : `enter_intro` détecte `EditorTestEnemy` et
  fait `NextState::set(LevelPhase::Running)` immédiatement (sans `IntroData`).
- Le `countdown` global est aussi skippé en éditeur.

Pour ajouter un ennemi/scénario testable dans l'éditeur :
1. Ajouter une entrée dans `EDITOR_ENEMIES` ([src/menu/mainmenu.rs](src/menu/mainmenu.rs)).
2. Si besoin d'une timeline custom (ex: simple_ufo wave demo), ajouter une
   branche dans `build_level_test_enemy` ([src/level/level.rs](src/level/level.rs)).

## Debug

- **F1** : toggle overlay + invulnérabilité player (composant `DebugInvulnerable`).
- **F2** : skip jusqu'à `pre_boss` dans la timeline.
- **F3** : skip l'intro.
- **F4** : win instantané — `debug_skip_to_outro` despawn les astéroïdes,
  marque `level_complete`, `NextState::set(LevelPhase::Outro)` directement
  (skip le countdown).
- **F5** : tue le joueur (test GameOver).
