# Framework ennemi

Machine à état générique dans [src/enemy/](src/enemy/) — réutilisable par tous les ennemis. Chaque ennemi en utilise une partie selon sa complexité.

## États

```
Entering ──→ Flexing ──→ Idle ──→ Active(0) ──┬──→ Transitioning(1) ──→ Active(1) ──→ …
                                               └──→ Dying ──→ Dead
```

| État | Description | Vulnérable | Dangereux |
|------|-------------|------------|-----------|
| `Entering` | Animation d'arrivée (optionnelle) | Non | Non |
| `Flexing` | Animation post-arrivée (optionnelle) | Non | Non |
| `Idle` | Attente, animation idle, immobile | Non | Non |
| `Active(n)` | Phase n, patterns actifs | Oui | Oui |
| `Transitioning(n)` | Transition vers phase n (shake + flash). Géré par le module de l'ennemi. | Non | Non |
| `Dying` | Animation de mort (shake, flash, explosions) | Non | Non |
| `Dead` | Despawné | — | — |

**Phases** : liste de `PhaseDef` dans [enemies.rs](src/enemy/enemies.rs). PV propres, patterns, flag `has_transition`. PV à 0 → `Transitioning(next)` si `has_transition=true`, sinon `Active(next)` direct ou `Dying` si dernière phase.

**Patterns** : `PatternDef { name, duration }`. Le pattern executor cycle dans la liste. Peut se terminer sur événement (ex: charge boss → fin au mur).

**Systèmes génériques** (dans `EnemyPlugin`) : dégâts missile, flash hit, transitions de phase, animation de mort, mouvement patrol sinusoïdal, déplacement projectiles ennemis.

**La machine à état est entièrement optionnelle.** Exemples :

| Ennemi | États utilisés | Mort |
|--------|---------------|------|
| **Boss** | `Entering` → `Flexing` → `Idle` → `Active(0)` → `Transitioning(1)` → `Active(1)` → `Transitioning(2)` → `Active(2)` → `Dying` → `Dead` | Longue (4s, shake + explosions + flexing accéléré) |
| **GreenUFO** | `Active(0)` → `Dying` → `Dead` | Instantanée |
| **Mine** | Pilotée par BehaviorTree (falling → counting_down → exploding) | Instantanée + AOE |
| **Kamikaze** | Pilotée par BehaviorTree (pursuing → armed → booming) | Instantanée + AOE |

## Ajouter un nouvel ennemi

1. Définir `PhaseDef` et `EnemyDef` dans [enemies.rs](src/enemy/enemies.rs)
2. Créer un module avec ses systèmes spécifiques (intro, patterns)
3. Si `has_transition = true`, implémenter les systèmes de transition dans le module
4. Les systèmes génériques fonctionnent automatiquement

Pour un ennemi simple : spawn direct en `Active(0)`, pas besoin de Entering/Flexing.

## Boss — Difficulté progressive

3 phases × 100 PV = 300 PV. Valeurs dans des tableaux indexés par phase (`BOSS_CHARGE_SPEEDS`, `BOSS_PATROL_SPEEDS_X`, `BOSS_TRANSITION_UFO_COUNT`) dans [boss.rs](src/enemy/boss.rs).

| Paramètre | Phase 1 | Phase 2 | Phase 3 |
|-----------|---------|---------|---------|
| Vitesse de charge | 1500 | 2000 | 2500 |
| Vitesse de patrol X | 200 | 270 | 270 |
| Durée patrol avant charge | 5.0s | 4.0s | 3.0s |
| UFOs en transition | — | 2 | 4 |

## Kamikaze — Pilotage par BehaviorTree

[src/enemy/kamikaze.rs](src/enemy/kamikaze.rs). 3 phases d'`alive_choice` + outer dying :

1. **pursuing** : Chase + animation `kamikaze_chase` (frames 000→003, one-shot, reste sur 003)
2. **armed** : Chase continue + BlinkRed + animation `kamikaze_explode` (frames 004→014, one-shot). Marker `KamikazeArmed` → `kamikaze_warn_system` joue `Sfx::KamikazeWarn` à l'entrée. Durée alignée sur l'animation. `on_complete("boom")` → phase suivante.
3. **booming** : marker `KamikazeBoom` → `kamikaze_boom_system` spawn AOE + son + despawn

**Sprites** : sous-dossiers [assets/images/kamikaze/chase/](assets/images/kamikaze/chase/) (4 frames warm-up) et [explode/](assets/images/kamikaze/explode/) (11 frames boom). Chaque dossier = une animation autonome dans `AnimBank`, jouée en mode `one_shot()` (s'arrête sur la dernière frame).

**KamikazeSpeedRamp** : composant qui ajoute un déplacement additionnel vers le joueur dont la magnitude grandit avec l'âge (`bonus = age × RAMP_RATE`, plafonné à `MAX_BONUS`). S'additionne au `Chase` de base — un kamikaze "ancien" devient plus rapide que le joueur, ce qui force à le prioriser.

**Mort silencieuse par tirs** : si tué (HP=0) avant la fin du countdown, outer choice → DespawnSelf, pas d'AOE.
