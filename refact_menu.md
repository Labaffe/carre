# Analyse — Système de menus, pause et navigation

## 1. Cartographie de l'état actuel

### États déclarés (`state.rs`)

```
MainMenu ─┬─→ LevelSelect ─┬─→ Playing ─┬─→ LevelTransition ─→ LevelSelect | Credits
          │                │            ├─→ GameOver ─→ Playing | MainMenu
          │                │            └─→ MainMenu (via pause)
          │                └─→ MainMenu
          ├─→ Settings (sous-vue cachée dans MainMenuAnim, PAS un état)
          └─→ ...
```

### "Pseudo-états" cachés dans des ressources

| État réel | Pseudo-état | Source |
|-----------|-------------|--------|
| `GameState` (Bevy) | — | `state.rs` |
| **MenuView** (`Main`/`Settings`) | resource `MainMenuAnim.view` | `mainmenu.rs:80-85` |
| **PauseState.paused** | bool dans resource | `pause.rs:43-51` |
| **PauseState.intro_active** | bool dans resource | idem |
| **PauseState.outro_active** | bool dans resource | idem |
| **ConfirmPopup** | resource ad-hoc | `game.rs:96` |

→ Au moins **5 systèmes d'état parallèles** se mélangent. C'est la racine du désordre.

---

## 2. Problèmes concrets

### 🔴 P1. Code de la popup de confirmation dupliqué **3 fois**

Exactement la même logique (navigation gauche/droite, mise à jour couleurs Oui/Non, Enter, Escape, despawn, NextState, remove resources) est copiée-collée dans :

| Fichier | Lignes | Action sur "Oui" |
|---------|--------|------------------|
| `pause.rs:90-135` | ~45 lignes | remove `CampaignProgress` + `PlayMode` + `ConfirmPopup` + unpause + → MainMenu |
| `levelselect.rs:369-410` | ~42 lignes | remove `CampaignProgress` + `PlayMode` + `ConfirmPopup` + → MainMenu |
| `gameover.rs:322-362` | ~41 lignes | remove `CampaignProgress` + `PlayMode` + `ConfirmPopup` + → MainMenu |

**~130 lignes de copie**. Si tu changes la popup (ajouter un bouton, changer le wording, ajouter un son), il faut modifier 3 fichiers. Et tu peux oublier un `commands.remove_resource::<X>()` dans une copie.

### 🔴 P2. Cleanup des resources de session éparpillé sur N chemins

`PlayMode` et `CampaignProgress` sont insérés à un seul endroit (`mainmenu.rs:472-479`) mais retirés à **6 endroits différents** :

```
pause.rs:115-117      (popup oui)
pause.rs:232          (Primes → MainMenu)
levelselect.rs:394-396 (popup oui)
levelselect.rs:466    (Primes → Escape)
levelselect.rs:475    (Campaign sans progression → Escape)
gameover.rs:273-274   (fin du fade campagne)
gameover.rs:345-347   (popup oui)
gameover.rs:377       (Primes → Escape)
```

Si tu ajoutes un nouveau chemin (ex: GameOver → Credits direct), il faut **se rappeler** de retirer ces resources. Sinon état inconsistant.

### 🔴 P3. La pause n'est pas un état — c'est un bool

`PauseState { paused, intro_active, outro_active, selected }` est un cocktail :
- 3 booléens qui sont en réalité **un enum** : `Running | Paused | LevelIntro | LevelOutro | ConfirmingAbandon`
- + un `selected` pour la nav
- + une popup parallèle (`ConfirmPopup`)

Le système `not_paused()` (`pause.rs:37`) doit faire `!paused && !intro_active && !outro_active`. Trois comparaisons pour un truc qui devrait être `state == Running`. L'oubli d'un flag dans la run condition = bug subtil.

### 🔴 P4. `MenuView::Settings` est caché à l'intérieur du MainMenu

```rust
struct MainMenuAnim { elapsed, selected, view: MenuView }
enum MenuView { Main, Settings }
```

Conséquences :
- L'UI n'est pas vraiment despawnée quand on entre dans Settings — on la cache avec `style.display = Display::None` (`mainmenu.rs:354-369`).
- Pas de OnEnter/OnExit pour Settings.
- Si demain tu veux des Settings accessibles depuis la pause aussi, il faut tout dupliquer.
- Le sous-menu n'apparaît pas dans `state.rs` → invisible pour qui lit l'archi.

### 🔴 P5. Logique de musique fragmentée

| Lieu | Action sur la musique du menu |
|------|-------------------------------|
| `mainmenu.rs:159-166` | spawn si absente |
| `levelselect.rs:137-144` | spawn si absente (code copié) |
| `levelselect.rs:450-453` | despawn quand on lance un niveau |
| `pause.rs:144-150 / 159-164 / 213-218` | pause/play sinks (2× fois) |
| `gameover.rs:154-157` | despawn `MusicMain` |
| `boss.rs` | gère sa propre musique |
| `game.rs` | `start_outro()` despawn `MusicMain` + `MusicBoss` |

Pas un seul module ne peut dire "qui est responsable de la musique". Quand on fait MainMenu → LevelSelect, la musique survit "par accident" parce que les deux setup vérifient son existence.

### 🔴 P6. Les transitions ne sont validées nulle part

Bevy permet `next_state.set(GameState::X)` depuis n'importe quel système. Aucun garde-fou. Conséquence directe : un système peut faire `set(MainMenu)` pendant une pause sans nettoyer la pause, sans nettoyer la popup, sans rien.

Exemple potentiel : Echap dans la popup de confirmation pendant la pause → ferme juste la popup. Mais si la popup est buggée et n'est jamais affichée, Echap fait rien. Si c'est Enter qui valide "Oui", il faut espérer que tous les remove_resource sont là.

### 🔴 P7. `LevelTransition` est un état artificiel

C'est un **passage forcé** pour déclencher le cleanup d'OnExit(Playing). Mais il n'a aucun setup/update propre — c'est juste un trampoline. Pattern fragile :
- Si quelqu'un set `LevelTransition` → `MainMenu` directement sans passer par `LevelSelect`, le cleanup ne saurait pas distinguer.
- Difficile à raisonner pour un nouveau dev.

### 🔴 P8. Mélange UI et logique partout

Chaque fichier menu fait :
1. Spawn UI (Bundle, ImageBundle, TextBundle, NodeBundle…) — **lourd**, ~80 lignes par menu
2. Anim (couleurs, alpha) — **40 lignes**
3. Input (KeyCode, navigation) — **50 lignes**
4. Cleanup — **15 lignes**

Total : ~200 lignes par menu, dont **80 % sont identiques** (couleur sélectionné = jaune, non-sélectionné = gris, navigation up/down/Enter/Escape).

### 🔴 P9. Cleanup du LevelSelect — hack visible

`levelselect.rs:486-497` :
```rust
ui_q: Query<(Entity, Option<&Parent>), With<LevelSelectUI>>,
// Ne despawn que les entités racine — les enfants suivent via despawn_recursive
for (entity, parent) in ui_q.iter() {
    if parent.is_none() { ... }
}
```

Indique que `LevelSelectUI` a été mis à la fois sur le root **et** sur des enfants (probablement par erreur ou par sécurité). Le hack contourne le double-despawn.

### 🔴 P10. Couplage `pause.rs` → `game::ConfirmPopup`

`pause.rs` importe explicitement `ConfirmPopup`, `ConfirmPopupUI`, `spawn_confirm_popup`, `despawn_confirm_popup` depuis `game.rs`. Idem pour `levelselect.rs` et `gameover.rs`. La popup n'a pas son propre module.

---

## 3. Architecture proposée

### Idée 1 : Une seule machine d'état, hiérarchique et explicite

```rust
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    MainMenu,
    Settings,           // PROMU en vrai état (plus de MenuView caché)
    LevelSelect,
    LevelIntro,         // PROMU (au lieu de PauseState.intro_active)
    Playing,
    Paused,             // PROMU (au lieu de PauseState.paused)
    LevelOutro,         // PROMU (au lieu de PauseState.outro_active)
    LevelTransition,
    GameOver,
    Credits,
}

// Modal orthogonal à l'état principal
#[derive(Resource, Default)]
pub enum Modal {
    #[default]
    None,
    Confirm(ConfirmDesc),
    // futur : Notification, Loading, etc.
}
```

Bénéfices immédiats :
- `not_paused()` devient `in_state(AppState::Playing)` — pas besoin de la run condition custom
- Plus de `intro_active`/`outro_active`/`paused` à juggler
- Bevy gère les transitions, OnEnter, OnExit pour chaque

### Idée 2 : Modal réutilisable, dans son propre module

```rust
// src/ui/modal.rs
pub struct ModalPlugin;

#[derive(Clone)]
pub struct ConfirmDesc {
    pub title: &'static str,
    pub message: &'static str,
    pub yes_label: &'static str,
    pub no_label: &'static str,
    pub on_yes: ModalAction,
    pub on_no: ModalAction,  // souvent Close
}

#[derive(Clone)]
pub enum ModalAction {
    Close,
    GoTo(AppState),
    GoToWith { state: AppState, abandon_session: bool },
    Custom(fn(&mut Commands)),
}

impl Plugin for ModalPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Modal>()
            .add_systems(Update, (
                spawn_modal_ui_when_changed,
                handle_modal_input.run_if(modal_active),
                update_modal_colors.run_if(modal_active),
            ));
    }
}

// Helpers d'appel :
pub fn confirm_abandon_campaign() -> Modal {
    Modal::Confirm(ConfirmDesc {
        title: "Votre progression sera perdue.",
        message: "Continuer ?",
        yes_label: "Oui",
        no_label: "Non",
        on_yes: ModalAction::GoToWith { state: AppState::MainMenu, abandon_session: true },
        on_no:  ModalAction::Close,
    })
}
```

Usage :
```rust
// Dans pause.rs, levelselect.rs, gameover.rs : juste
*modal = confirm_abandon_campaign();
```

→ Les 130 lignes dupliquées disparaissent.

### Idée 3 : Resource `PlaySession` unifiée

```rust
#[derive(Resource)]
pub struct PlaySession {
    pub mode: PlayMode,
    pub campaign: Option<CampaignProgress>,
    pub current_level: usize,
}

impl PlaySession {
    pub fn campaign() -> Self { ... }
    pub fn primes() -> Self { ... }
}
```

Et un système central :
```rust
fn cleanup_session_on_modal_action(
    mut commands: Commands,
    mut events: EventReader<ModalActionEvent>,
) {
    for ev in events.read() {
        if matches!(ev.action, ModalAction::GoToWith { abandon_session: true, .. }) {
            commands.remove_resource::<PlaySession>();
        }
    }
}
```

→ Plus de `commands.remove_resource::<PlayMode>()` éparpillé.

### Idée 4 : Trait `Menu` + widget générique

```rust
pub trait Menu: Resource {
    type Action: Clone;

    fn layout(&self) -> MenuLayout;
    fn options(&self) -> Vec<MenuItem<Self::Action>>;
    fn selected(&self) -> usize;
    fn set_selected(&mut self, idx: usize);
    fn on_validate(&self, action: Self::Action, ctx: &mut MenuContext);
}

pub enum MenuLayout {
    Vertical { gap: f32, font_size: f32 },
    Horizontal { gap: f32, font_size: f32 },
    Grid { cols: usize, slot_size: Vec2 },
}

pub struct MenuItem<A> {
    pub label: String,
    pub icon: Option<Handle<Image>>,
    pub disabled: bool,
    pub action: A,
}

pub struct MenuContext<'a> {
    pub commands: Commands<'a, 'a>,
    pub next_state: ResMut<'a, NextState<AppState>>,
    pub modal: ResMut<'a, Modal>,
    pub session: Option<ResMut<'a, PlaySession>>,
}
```

Implémentations :
```rust
#[derive(Resource)]
pub struct MainMenuModel { selected: usize }

impl Menu for MainMenuModel {
    type Action = MainAction;
    fn layout(&self) -> MenuLayout { MenuLayout::Vertical { gap: 20.0, font_size: 36.0 } }
    fn options(&self) -> Vec<MenuItem<MainAction>> {
        vec![
            MenuItem { label: "Commencer".into(), action: MainAction::StartCampaign, ... },
            MenuItem { label: "Primes".into(),    action: MainAction::OpenPrimes,    ... },
            MenuItem { label: "Paramètres".into(), action: MainAction::OpenSettings,  ... },
            MenuItem { label: "Quitter".into(),   action: MainAction::Quit,          ... },
        ]
    }
    fn on_validate(&self, action: MainAction, ctx: &mut MenuContext) {
        match action {
            MainAction::StartCampaign => {
                ctx.commands.insert_resource(PlaySession::campaign());
                ctx.next_state.set(AppState::LevelSelect);
            }
            MainAction::OpenSettings => ctx.next_state.set(AppState::Settings),
            ...
        }
    }
}
```

Un seul système Bevy **`render_menu<M: Menu>`** + **`handle_menu_input<M: Menu>`** + **`cleanup_menu<M: Menu>`** générique.

→ MainMenu, Pause, GameOver, ConfirmModal, LevelSelect deviennent **chacun ~30 lignes** (juste l'impl du trait), au lieu de 200.

### Idée 5 : Service de navigation centralisé

```rust
#[derive(Event)]
pub enum NavEvent {
    GoTo(AppState),
    GoToWithSession(AppState, PlaySession),
    AbandonAndGoTo(AppState),
    Pop,  // retour au précédent
}

#[derive(Resource, Default)]
pub struct NavStack(Vec<AppState>);

fn navigation_handler(
    mut events: EventReader<NavEvent>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<AppState>>,
    mut stack: ResMut<NavStack>,
    current_state: Res<State<AppState>>,
) {
    for ev in events.read() {
        match ev {
            NavEvent::GoTo(s) => {
                stack.0.push(*current_state.get());
                next_state.set(*s);
            }
            NavEvent::AbandonAndGoTo(s) => {
                commands.remove_resource::<PlaySession>();
                stack.0.clear();
                next_state.set(*s);
            }
            NavEvent::Pop => {
                if let Some(prev) = stack.0.pop() {
                    next_state.set(prev);
                }
            }
            ...
        }
    }
}
```

→ La navigation devient un **événement** : impossible de casser l'état global.
→ `NavStack` permet "Echap = retour", utile pour Settings.

### Idée 6 : Audio centralisé

```rust
#[derive(Resource)]
pub struct AudioDirector {
    main_menu: Option<Entity>,
    gameplay:  Option<Entity>,
    boss:      Option<Entity>,
    outro:     Option<Entity>,
    gameover:  Option<Entity>,
}

impl AudioDirector {
    pub fn play_menu(&mut self, ...) { ... }
    pub fn stop_menu(&mut self, ...) { ... }
    pub fn pause_all(&self) { ... }
    pub fn resume_all(&self) { ... }
}

// Système réactif aux changements d'état
fn audio_director_on_state_enter(
    mut director: ResMut<AudioDirector>,
    state: Res<State<AppState>>,
    ...
) {
    match state.get() {
        AppState::MainMenu | AppState::LevelSelect | AppState::Settings => {
            director.play_menu(...);
        }
        AppState::Playing => {
            director.stop_menu(...);
        }
        AppState::Paused => {
            director.pause_all();
        }
        ...
    }
}
```

→ Plus aucun `commands.spawn(AudioBundle{...})` dans pause/levelselect/gameover.

---

## 4. Cleanup générique par marker

```rust
// Marker "vit pendant cet état"
#[derive(Component)]
pub struct LiveDuring(pub AppState);

// Système unique installé dans Bevy
fn cleanup_on_state_exit(
    mut commands: Commands,
    state: Res<State<AppState>>,
    q: Query<(Entity, &LiveDuring)>,
) {
    let leaving = state.get();
    for (entity, live) in q.iter() {
        if live.0 == *leaving {
            if let Some(e) = commands.get_entity(entity) {
                e.despawn_recursive();
            }
        }
    }
}
```

→ Plus besoin d'écrire `cleanup_main_menu`, `cleanup_pause`, `cleanup_gameover_ui`, `cleanup_level_select`. **Un seul système** pour tout.

---

## 5. Résumé visuel : avant / après

### Avant (état actuel)

```
┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│  mainmenu   │  │  pause      │  │ levelselect │  │  gameover   │
│  600 LOC    │  │  370 LOC    │  │  500 LOC    │  │  390 LOC    │
│ ─────────── │  │ ─────────── │  │ ─────────── │  │ ─────────── │
│ MenuView    │  │ PauseState  │  │ LSState     │  │ GameOverAnim│
│ Anim sys    │  │ ConfirmPopup│  │ ConfirmPopup│  │ ConfirmPopup│
│ Input sys   │  │  (dupliqué) │  │  (dupliqué) │  │  (dupliqué) │
│ Settings UI │  │ Music sinks │  │ Music spawn │  │ Music stop  │
│ Music spawn │  │ remove_res  │  │ remove_res  │  │ remove_res  │
│ Cleanup     │  │ unpause     │  │ Cleanup hack│  │ Cleanup     │
└─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘
                          ↓ tous accèdent à
              ┌──────────────────────────────────┐
              │ PlayMode + CampaignProgress       │
              │ (insérés à 1 lieu, retirés à 6)   │
              └──────────────────────────────────┘
```

### Après (cible)

```
┌──────────────────────────────────────────────────────────────────┐
│ AppState (10 variants explicites)                                │
│ MainMenu | Settings | LevelSelect | LevelIntro | Playing |       │
│ Paused | LevelOutro | LevelTransition | GameOver | Credits       │
└──────────────────────────────────────────────────────────────────┘
       │
       ├── Modal (resource)    : popup unique, 1 module
       ├── PlaySession (res)   : mode + campaign + level, 1 source
       ├── NavStack (res)      : back/forward, 1 service
       ├── AudioDirector (res) : musique selon état, 1 service
       │
       ▼
┌──────────────────────────────────────────────────────────────────┐
│ Trait Menu : MainMenuModel, PauseMenu, GameOverMenu, etc.        │
│ ~30 LOC chacun (juste les options et les actions)                │
└──────────────────────────────────────────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────────────────────────────────┐
│ Systèmes génériques :                                            │
│  - render_menu<M>  - handle_menu_input<M>  - cleanup_menu<M>     │
│  - cleanup_on_state_exit (via LiveDuring marker)                 │
│  - navigation_handler (consomme NavEvent)                        │
│  - modal_handler                                                 │
└──────────────────────────────────────────────────────────────────┘
```

**LOC estimées :** 1 860 → ~600 (–67 %)

---

## 6. Plan de migration progressif

| # | Étape | Risque | Bénéfice |
|---|-------|--------|----------|
| 1 | Extraire `Modal` + `ModalPlugin` (1 seul module). Migrer pause, levelselect, gameover pour appeler `*modal = confirm_abandon_campaign()` | Faible | –130 LOC dupliquées |
| 2 | Créer `PlaySession` qui englobe `PlayMode + CampaignProgress`. Adapter les 6 lieux de remove. | Faible | Plus jamais d'oubli de cleanup |
| 3 | Promouvoir `Settings`, `Paused`, `LevelIntro`, `LevelOutro` en vrais `AppState`. Supprimer les bools de `PauseState` | Moyen | `not_paused()` devient `in_state(Playing)` |
| 4 | Introduire `NavEvent` + `NavStack`. Remplacer `next_state.set(...)` par `nav.send(...)` | Moyen | Centralise toutes les transitions |
| 5 | Trait `Menu` + systèmes génériques. Migrer un menu à la fois (commencer par Pause, le plus simple) | Élevé | Chaque menu = 30 LOC |
| 6 | `AudioDirector` réactif aux états | Moyen | Plus de bugs de musique fantôme |
| 7 | Marker `LiveDuring` + cleanup générique. Supprimer les 6 fonctions cleanup | Faible | –50 LOC |

**Tu peux t'arrêter à l'étape 2 et déjà avoir gagné énormément** (popup unique + session unifiée).

---

## TL;DR des problèmes

1. **3 systèmes d'état parallèles** (Bevy `GameState`, `MainMenuAnim.view`, `PauseState.{paused,intro_active,outro_active}`) qui interagissent par accident
2. **130 LOC dupliquées** pour la popup de confirmation
3. **`PlayMode` retiré à 6 endroits** dispersés — point de friction majeur
4. **`Settings` n'est pas un état** — sous-vue cachée, impossible à réutiliser
5. **Aucune validation des transitions** entre états
6. **Audio piloté par 6 modules différents** sans chef d'orchestre
7. **80 % du code des menus est dupliqué** (animation couleurs, navigation up/down/Enter, cleanup)
8. **`LevelTransition` est un trampoline fragile** plutôt qu'une vraie phase

La solution tient en quatre primitives : `AppState` plus riche, resource `Modal`, resource `PlaySession`, trait `Menu`. Le reste découle.