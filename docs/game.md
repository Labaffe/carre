# Game manager — Progression et outro

[src/game_manager/game.rs](src/game_manager/game.rs) gère la progression multi-niveaux et la séquence d'outro (victoire).

## GameState

```
MainMenu → Playing → LevelTransition → Playing → … → MainMenu
                  └→ GameOver → MainMenu
```

| État | Description |
|------|-------------|
| `MainMenu` | Menu principal (Commencer, Niveaux, Paramètres, Quitter) |
| `Playing` | Niveau en cours |
| `LevelTransition` | État transitoire entre deux niveaux (déclenche cleanup puis setup) |
| `GameOver` | Écran de game over |

## LevelPhaseKind

Sous-état dans `Playing`, géré par `level_phase_system` :

| Phase | Effet |
|-------|-------|
| `Intro` | Animation d'arrivée du vaisseau + son. Freeze gameplay via `pause.intro_active=true`. Termine quand son ET animation finis. Skippée en mode éditeur. |
| `Running` | LevelRunner exécute les étapes. |
| `OutroCountdown` | 3s après mort du boss avant outro. |
| `Outro` | Freeze + musique victoire + texte. Entrée/Espace → niveau suivant ou MainMenu. |

## GameProgress

```rust
pub struct GameProgress {
    pub current_level: usize,  // 1-indexed
    pub total_levels: usize,
}
```

- « Commencer » → niveau 1
- Menu « Niveaux » → sélection directe
- `setup_level` lit `current_level` → choisit le builder (`build_level_1()`, etc.)

## Outro — Séquence de victoire

```
Boss meurt → 3s OutroCountdown → Outro (freeze + musique + texte)
  → Entrée/Espace → LevelTransition (suivant)
                   ou MainMenu (dernier niveau)
```

1. **Détection** : `detect_level_complete` (`boss_spawned && boss_q.is_empty()`)
2. **Countdown** : 3s
3. **`start_outro()`** — freeze via `PauseState.outro_active = true`, despawn musiques, stop background, spawn UI victoire
4. **Musique** : `stage_clear.ogg` une seule fois
5. **Input** : après 3s de délai, Entrée/Espace continue

## Mode éditeur

Ressource `EditorTestEnemy(name)` insérée par le menu éditeur. Quand présente au moment de `setup_level` :
- Timeline minimale : spawn 1 ennemi du type demandé à `SpawnPosition::At(0.0, 200.0)`
- Phase `Intro` skippée → démarre direct en `Running`
- Joueur démarre en Phase3 (vitesse 1000, projectiles bleus)
- Gate `difficulty.elapsed < 1.0` du mouvement joueur bypassée

## Debug

- F4 → win instantané : tue tous les ennemis et appelle `start_outro()` sans countdown
