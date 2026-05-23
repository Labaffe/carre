//! Progression du jeu — machine à état du niveau via Bevy `SubState`.
//!
//! ## Flow
//!
//! ```text
//! [Default: Intro (optionnelle)] → Running → OutroCountdown → Outro
//! ```
//!
//! - `LevelPhase` est un `SubState` de `GameState::Playing` : il est
//!   automatiquement créé à l'entrée de Playing (variant Default = Intro)
//!   et retiré à la sortie. Plus besoin de `commands.remove_resource`.
//! - Chaque variante a sa Resource éphémère pour les données vivantes :
//!   `IntroData`, `OutroCountdownData`, `OutroData`. Insérée en `OnEnter`,
//!   retirée en `OnExit`. Les systèmes d'une phase peuvent donc supposer
//!   que leur data Resource existe (pas d'`Option<Res<…>>` partout).
//! - Les systèmes de chaque phase sont gated par `run_if(in_state(...))`.
//! - **Mode éditeur** : `enter_intro` détecte `EditorTestEnemy` et court-
//!   circuite directement vers Running (pas de données, pas de pause).

use std::collections::HashSet;

use crate::MusicMain;
use crate::enemy::asteroid::Asteroid;
use crate::enemy::boss::{BossMarker, MusicBoss};
use crate::enemy::enemy::Enemy;
use crate::game_manager::difficulty::Difficulty;
use crate::game_manager::state::GameState;
use crate::level::level::{EditorTestEnemy, LevelConfig, level_name};
use crate::level::levels::ScrollDirection;
use crate::menu::pause::PauseState;
use crate::player::player::Player;
use bevy::prelude::*;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameProgress>()
            .add_sub_state::<LevelPhase>()
            // ─── Intro ──────────────────────────────────────────────
            .add_systems(OnEnter(LevelPhase::Intro), enter_intro)
            .add_systems(OnExit(LevelPhase::Intro), exit_intro)
            .add_systems(
                Update,
                (intro_tick, skip_intro_input).run_if(in_state(LevelPhase::Intro)),
            )
            // ─── Running → OutroCountdown ───────────────────────────
            .add_systems(
                Update,
                (detect_boss_death, detect_level_complete)
                    .run_if(in_state(LevelPhase::Running)),
            )
            // ─── OutroCountdown ─────────────────────────────────────
            .add_systems(OnEnter(LevelPhase::OutroCountdown), enter_outro_countdown)
            .add_systems(OnExit(LevelPhase::OutroCountdown), exit_outro_countdown)
            .add_systems(
                Update,
                countdown_tick.run_if(in_state(LevelPhase::OutroCountdown)),
            )
            // ─── Outro ──────────────────────────────────────────────
            .add_systems(OnEnter(LevelPhase::Outro), enter_outro)
            .add_systems(OnExit(LevelPhase::Outro), exit_outro)
            .add_systems(
                Update,
                (level_outro_animate, level_outro_input).run_if(in_state(LevelPhase::Outro)),
            )
            // ─── F4 : skip direct à l'outro ─────────────────────────
            // Ordonné `after(detect_level_complete)` pour que son
            // `NextState::set(Outro)` ne soit pas écrasé par un
            // `NextState::set(OutroCountdown)` si `level_complete` était
            // déjà true au moment du F4.
            .add_systems(
                Update,
                debug_skip_to_outro
                    .after(detect_level_complete)
                    .run_if(in_state(GameState::Playing))
                    .run_if(not_in_outro),
            )
            .add_systems(OnExit(GameState::Playing), cleanup_playing)
            .add_systems(OnEnter(GameState::LevelTransition), auto_start_next_level)
            .add_systems(OnEnter(GameState::Credits), setup_credits)
            .add_systems(OnExit(GameState::Credits), cleanup_credits)
            .add_systems(
                Update,
                handle_credits_input.run_if(in_state(GameState::Credits)),
            );
    }
}

/// `run_if` helper : true tant que la sous-phase n'est pas `Outro`.
/// `Option<Res<State<…>>>` car `LevelPhase` n'existe pas hors de `Playing`.
fn not_in_outro(phase: Option<Res<State<LevelPhase>>>) -> bool {
    phase.map_or(false, |p| !matches!(*p.get(), LevelPhase::Outro))
}

// ─── Ressources ─────────────────────────────────────────────────────

/// Progression du jeu : quel niveau est en cours.
#[derive(Resource)]
pub struct GameProgress {
    pub current_level: usize,
    pub total_levels: usize,
}

impl Default for GameProgress {
    fn default() -> Self {
        Self {
            current_level: 1,
            total_levels: crate::level::levels::ALL_LEVELS.len(),
        }
    }
}

/// Mode de jeu actif. Inséré quand le joueur choisit Commencer ou Primes.
#[derive(Resource, Clone, Copy, PartialEq, Eq)]
pub enum PlayMode {
    /// Campagne : finir tous les niveaux, progression perdue si mort/quit.
    Campaign,
    /// Primes : jouer un niveau à la carte.
    Primes,
}

/// Progression de la campagne en cours. Supprimée si abandon ou fin.
#[derive(Resource, Default)]
pub struct CampaignProgress {
    pub completed: HashSet<usize>,
}

/// Popup de confirmation active (abandon campagne).
#[derive(Resource)]
pub struct ConfirmPopup {
    pub selected: usize, // 0 = Non, 1 = Oui
}

/// Marqueur pour les éléments UI de la popup de confirmation.
#[derive(Component)]
pub struct ConfirmPopupUI;

/// Marqueur pour les options Oui/Non de la popup (0=Non, 1=Oui).
#[derive(Component)]
pub struct ConfirmOptionMarker(pub usize);

// ═══════════════════════════════════════════════════════════════════════
//  Machine à état du niveau : SubState LevelPhase
// ═══════════════════════════════════════════════════════════════════════

/// Sous-phase du niveau en cours. `SubState` lié à `GameState::Playing` :
/// auto-créé à l'entrée, auto-supprimé à la sortie.
#[derive(SubStates, Debug, Clone, PartialEq, Eq, Hash, Default)]
#[source(GameState = GameState::Playing)]
pub enum LevelPhase {
    /// Animation d'entrée du vaisseau (optionnelle). En mode éditeur,
    /// `enter_intro` court-circuite directement vers Running.
    #[default]
    Intro,
    /// Niveau en cours : les `LevelStep` s'exécutent (cf. `run_level` dans level.rs).
    Running,
    /// Délai de 3s entre `level_complete` et l'outro.
    OutroCountdown,
    /// Séquence de victoire (musique + écran).
    Outro,
}

// ─── Données éphémères par phase ────────────────────────────────────

/// Données de la phase Intro. Insérée en `OnEnter(Intro)`, retirée en
/// `OnExit(Intro)`. Absente en mode éditeur (l'intro est court-circuitée).
#[derive(Resource)]
pub struct IntroData {
    pub elapsed: f32,
    pub duration: f32,
    pub sound: crate::audio::Sfx,
    pub sound_played: bool,
    /// Le son d'intro a fini de jouer (entité `IntroSound` despawnée).
    pub sound_finished: bool,
    /// Position de départ (hors écran).
    pub start_pos: Vec2,
    /// Position cible (en jeu).
    pub target_pos: Vec2,
    /// Ratio pour calculer la position cible (ex: -0.5 → 50% du half-screen).
    pub spawn_ratio: f32,
    pub initialized: bool,
}

/// Données du countdown avant l'outro.
#[derive(Resource)]
pub struct OutroCountdownData {
    pub timer: Timer,
}

/// Données de la phase Outro.
#[derive(Resource)]
pub struct OutroData {
    pub elapsed: f32,
    pub music_spawned: bool,
}

// ─── Composants ─────────────────────────────────────────────────────

/// Marqueur pour tous les éléments UI de l'outro. Auto-cleanup via le
/// marker `GameplayEntity` à la sortie de `GameState::Playing`.
#[derive(Component)]
#[require(crate::GameplayEntity)]
struct OutroUI;

/// Marqueur pour le son d'intro (landing.ogg). Despawné automatiquement
/// par Bevy quand la lecture est terminée (`PlaybackSettings::DESPAWN`),
/// ou par `do_skip_intro` si l'utilisateur skip.
#[derive(Component)]
#[require(crate::GameplayEntity)]
pub struct IntroSound;

/// Marqueur pour la musique de l'outro.
#[derive(Component)]
#[require(crate::GameplayEntity)]
pub struct MusicOutro;

// ─── Constantes ─────────────────────────────────────────────────────

/// Délai entre `level_complete` et le début de l'outro (secondes).
const OUTRO_COUNTDOWN: f32 = 3.0;
/// Délai minimum avant d'accepter l'input pour continuer (secondes).
const OUTRO_INPUT_DELAY: f32 = 3.0;

// ─── Configuration d'intro par niveau ───────────────────────────────

/// Configuration d'une intro de niveau.
pub struct IntroConfig {
    /// Durée de l'animation d'entrée du vaisseau (= durée du son).
    pub duration: f32,
    /// Son joué pendant l'intro.
    pub sound: crate::audio::Sfx,
    /// Ratio pour calculer la position cible.
    /// Pour Down : target_y = half_h * ratio (ex: -0.5 → bas de l'écran).
    /// Pour Left : target_x = half_w * ratio (ex: -0.5 → gauche de l'écran).
    pub spawn_ratio: f32,
}

/// Retourne la config d'intro pour un niveau.
pub fn level_intro(level: usize) -> IntroConfig {
    match level {
        _ => IntroConfig {
            duration: 5.0,
            sound: crate::audio::Sfx::ShipArrival,
            spawn_ratio: -0.5,
        },
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Phase: Intro
// ═══════════════════════════════════════════════════════════════════════

/// `OnEnter(LevelPhase::Intro)` : prépare la data + freeze le gameplay.
/// En mode éditeur, court-circuite vers Running sans rien insérer (la
/// transition au prochain frame n'aura donc aucun side-effect à nettoyer).
fn enter_intro(
    mut commands: Commands,
    progress: Res<GameProgress>,
    mut pause: ResMut<PauseState>,
    editor_test: Option<Res<EditorTestEnemy>>,
    mut next: ResMut<NextState<LevelPhase>>,
) {
    if editor_test.is_some() {
        next.set(LevelPhase::Running);
        return;
    }
    let intro = level_intro(progress.current_level);
    commands.insert_resource(IntroData {
        elapsed: 0.0,
        duration: intro.duration,
        sound: intro.sound,
        sound_played: false,
        sound_finished: false,
        start_pos: Vec2::ZERO,
        target_pos: Vec2::ZERO,
        spawn_ratio: intro.spawn_ratio,
        initialized: false,
    });
    pause.intro_active = true;
}

/// `OnExit(LevelPhase::Intro)` : libère la data + dégèle le gameplay.
/// `IntroData` peut être absente (mode éditeur où on a court-circuité).
fn exit_intro(mut commands: Commands, mut pause: ResMut<PauseState>) {
    commands.remove_resource::<IntroData>();
    pause.intro_active = false;
}

/// Anime le vaisseau pendant l'intro et transitionne vers Running quand
/// l'animation ET le son sont terminés.
fn intro_tick(
    time: Res<Time>,
    pause: Res<PauseState>,
    intro_data: Option<ResMut<IntroData>>,
    mut player_q: Query<&mut Transform, With<Player>>,
    windows: Query<&Window>,
    intro_sound_q: Query<Entity, With<IntroSound>>,
    config: Res<LevelConfig>,
    mut sfx: crate::audio::SfxPlayer,
    mut next: ResMut<NextState<LevelPhase>>,
) {
    // Peut être absente en mode éditeur (enter_intro a court-circuité avant
    // qu'OnExit n'ait nettoyé). On no-op proprement.
    let Some(mut data) = intro_data else { return };
    let window = windows.single().unwrap();
    let half_w = window.width() / 2.0;
    let half_h = window.height() / 2.0;

    // Premier frame : initialiser positions + rotation selon le scroll
    if !data.initialized {
        let (start, target) = match config.scroll_direction {
            ScrollDirection::Down => (
                Vec2::new(0.0, -half_h - 150.0),
                Vec2::new(0.0, half_h * data.spawn_ratio),
            ),
            ScrollDirection::Up => (
                Vec2::new(0.0, half_h + 150.0),
                Vec2::new(0.0, -(half_h * data.spawn_ratio)),
            ),
            ScrollDirection::Left => (
                Vec2::new(-half_w - 150.0, 0.0),
                Vec2::new(half_w * data.spawn_ratio, 0.0),
            ),
            ScrollDirection::Right => (
                Vec2::new(half_w + 150.0, 0.0),
                Vec2::new(-(half_w * data.spawn_ratio), 0.0),
            ),
        };
        data.start_pos = start;
        data.target_pos = target;
        data.initialized = true;

        let ship_angle = match config.scroll_direction {
            ScrollDirection::Down => 0.0,
            ScrollDirection::Up => std::f32::consts::PI,
            ScrollDirection::Left => -std::f32::consts::FRAC_PI_2,
            ScrollDirection::Right => std::f32::consts::FRAC_PI_2,
        };
        if let Ok(mut transform) = player_q.single_mut() {
            transform.translation.x = data.start_pos.x;
            transform.translation.y = data.start_pos.y;
            transform.rotation = Quat::from_rotation_z(ship_angle);
        }
        // Early return : son + anim démarrent au prochain frame. Laisse une
        // frame au `LoadingUI` pour se despawn + render avant l'audio
        // d'atterissage, sinon chevauchement "écran de chargement visible
        // + son qui démarre".
        return;
    }

    if pause.paused {
        return;
    }

    if !data.sound_played {
        data.sound_played = true;
        sfx.play(data.sound).insert(IntroSound);
    }
    if data.sound_played && !data.sound_finished && intro_sound_q.is_empty() {
        data.sound_finished = true;
    }

    data.elapsed += time.delta_secs();
    let anim_t = (data.elapsed / data.duration).clamp(0.0, 1.0);
    let eased = 1.0 - (1.0 - anim_t).powi(2);

    if let Ok(mut transform) = player_q.single_mut() {
        let pos = data.start_pos + (data.target_pos - data.start_pos) * eased;
        transform.translation.x = pos.x;
        transform.translation.y = pos.y;
    }

    if anim_t >= 1.0 && data.sound_finished {
        if let Ok(mut transform) = player_q.single_mut() {
            transform.translation.x = data.target_pos.x;
            transform.translation.y = data.target_pos.y;
        }
        next.set(LevelPhase::Running);
    }
}

/// Skip l'intro avec Entrée, Espace ou clic gauche.
fn skip_intro_input(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    intro_data: Option<Res<IntroData>>,
    pause: Res<PauseState>,
    mut player_q: Query<&mut Transform, With<Player>>,
    intro_sound_q: Query<Entity, With<IntroSound>>,
    windows: Query<&Window>,
    config: Res<LevelConfig>,
    mut next: ResMut<NextState<LevelPhase>>,
) {
    if !keyboard.just_pressed(KeyCode::Enter)
        && !keyboard.just_pressed(KeyCode::Space)
        && !mouse.just_pressed(MouseButton::Left)
    {
        return;
    }
    let Some(data) = intro_data else { return };
    if pause.paused {
        return;
    }
    do_skip_intro(
        &mut commands,
        &data,
        &mut next,
        &mut player_q,
        &intro_sound_q,
        &windows,
        &config,
    );
}

/// Place le joueur à sa position cible, despawn le son, transitionne vers Running.
/// Helper appelé depuis `skip_intro_input` et `debug_skip_intro` (debug.rs).
pub(crate) fn do_skip_intro(
    commands: &mut Commands,
    intro_data: &IntroData,
    next: &mut NextState<LevelPhase>,
    player_q: &mut Query<&mut Transform, With<Player>>,
    intro_sound_q: &Query<Entity, With<IntroSound>>,
    windows: &Query<&Window>,
    config: &Res<LevelConfig>,
) {
    let final_pos = if intro_data.initialized {
        intro_data.target_pos
    } else {
        let window = windows.single().unwrap();
        let half_w = window.width() / 2.0;
        let half_h = window.height() / 2.0;
        match config.scroll_direction {
            ScrollDirection::Down => Vec2::new(0.0, half_h * intro_data.spawn_ratio),
            ScrollDirection::Up => Vec2::new(0.0, -(half_h * intro_data.spawn_ratio)),
            ScrollDirection::Left => Vec2::new(half_w * intro_data.spawn_ratio, 0.0),
            ScrollDirection::Right => Vec2::new(-(half_w * intro_data.spawn_ratio), 0.0),
        }
    };
    if let Ok(mut transform) = player_q.single_mut() {
        transform.translation.x = final_pos.x;
        transform.translation.y = final_pos.y;
    }
    for entity in intro_sound_q.iter() {
        if let Ok(mut e) = commands.get_entity(entity) {
            e.try_despawn();
        }
    }
    next.set(LevelPhase::Running);
}

// ═══════════════════════════════════════════════════════════════════════
//  Phase: Running → OutroCountdown
// ═══════════════════════════════════════════════════════════════════════

/// Détecte la fin de l'animation de mort du dernier boss et envoie
/// `MarkLevelComplete` via le pipeline du niveau.
/// Pour les niveaux sans boss, `MarkLevelComplete` dans la timeline fait le travail.
fn detect_boss_death(
    mut difficulty: ResMut<Difficulty>,
    boss_q: Query<&Enemy, With<BossMarker>>,
    mut level_events: MessageWriter<crate::level::level::LevelActionEvent>,
) {
    if !difficulty.boss_seen_alive && !boss_q.is_empty() {
        difficulty.boss_seen_alive = true;
    }
    if difficulty.boss_seen_alive && boss_q.is_empty() && !difficulty.level_complete {
        level_events.write(crate::level::level::LevelActionEvent(vec![
            crate::level::level::Action::MarkLevelComplete,
        ]));
    }
}

/// Running → OutroCountdown dès que `difficulty.level_complete` est vrai.
fn detect_level_complete(
    difficulty: Res<Difficulty>,
    mut next: ResMut<NextState<LevelPhase>>,
) {
    if difficulty.level_complete {
        next.set(LevelPhase::OutroCountdown);
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Phase: OutroCountdown
// ═══════════════════════════════════════════════════════════════════════

fn enter_outro_countdown(mut commands: Commands) {
    commands.insert_resource(OutroCountdownData {
        timer: Timer::from_seconds(OUTRO_COUNTDOWN, TimerMode::Once),
    });
}

fn exit_outro_countdown(mut commands: Commands) {
    commands.remove_resource::<OutroCountdownData>();
}

fn countdown_tick(
    time: Res<Time>,
    mut data: ResMut<OutroCountdownData>,
    mut next: ResMut<NextState<LevelPhase>>,
) {
    data.timer.tick(time.delta());
    if data.timer.is_finished() {
        next.set(LevelPhase::Outro);
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Phase: Outro
// ═══════════════════════════════════════════════════════════════════════

/// `OnEnter(LevelPhase::Outro)` : freeze le jeu, coupe les musiques de
/// gameplay, stoppe le background, insère `OutroData`, spawn l'UI victoire.
fn enter_outro(
    mut commands: Commands,
    mut pause: ResMut<PauseState>,
    mut difficulty: ResMut<Difficulty>,
    asset_server: Res<AssetServer>,
    music_q: Query<Entity, With<MusicMain>>,
    boss_music_q: Query<Entity, With<MusicBoss>>,
    progress: Res<GameProgress>,
) {
    pause.outro_active = true;
    for entity in music_q.iter() {
        if let Ok(mut e) = commands.get_entity(entity) {
            e.try_despawn();
        }
    }
    for entity in boss_music_q.iter() {
        if let Ok(mut e) = commands.get_entity(entity) {
            e.try_despawn();
        }
    }
    difficulty.bg_speed_override = Some(0.0);
    commands.insert_resource(OutroData {
        elapsed: 0.0,
        music_spawned: false,
    });
    spawn_outro_ui(&mut commands, &asset_server, &progress);
}

fn exit_outro(mut commands: Commands, mut pause: ResMut<PauseState>) {
    commands.remove_resource::<OutroData>();
    pause.outro_active = false;
}

/// Tick l'elapsed + spawn la musique d'outro au premier passage.
fn level_outro_animate(
    mut commands: Commands,
    time: Res<Time>,
    mut data: ResMut<OutroData>,
    asset_server: Res<AssetServer>,
) {
    data.elapsed += time.delta_secs();
    if !data.music_spawned {
        data.music_spawned = true;
        commands.spawn((
            (
                AudioPlayer::new(asset_server.load("audio/music/stage_clear.ogg")),
                PlaybackSettings::ONCE,
            ),
            MusicOutro,
        ));
    }
}

/// Input pendant l'outro : Entrée/Espace → transition vers l'état suivant
/// selon le `PlayMode`. Le cleanup UI/musique se fait automatiquement via
/// `cleanup_playing` + `exit_outro` quand on quitte `GameState::Playing`.
fn level_outro_input(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    data: Res<OutroData>,
    mut next_state: ResMut<NextState<GameState>>,
    progress: Res<GameProgress>,
    play_mode: Option<Res<PlayMode>>,
    mut campaign: Option<ResMut<CampaignProgress>>,
) {
    if data.elapsed < OUTRO_INPUT_DELAY {
        return;
    }
    if !(keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space)) {
        return;
    }
    let level = progress.current_level;
    let mode = play_mode.map(|m| *m);
    match mode {
        Some(PlayMode::Campaign) => {
            if let Some(ref mut camp) = campaign {
                camp.completed.insert(level);
                if camp.completed.len() >= progress.total_levels {
                    commands.remove_resource::<CampaignProgress>();
                    commands.remove_resource::<PlayMode>();
                    next_state.set(GameState::Credits);
                } else {
                    next_state.set(GameState::LevelSelect);
                }
            }
        }
        Some(PlayMode::Primes) => {
            next_state.set(GameState::LevelSelect);
        }
        None => {
            next_state.set(GameState::MainMenu);
        }
    }
}

// ─── F4 : skip direct à l'outro ─────────────────────────────────────

/// F4 : tue tous les astéroïdes et transitionne directement vers l'outro.
/// `exit_intro`/`exit_outro_countdown` s'occupent du cleanup des phases
/// précédentes automatiquement.
fn debug_skip_to_outro(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut difficulty: ResMut<Difficulty>,
    asteroid_q: Query<Entity, With<Asteroid>>,
    mut next: ResMut<NextState<LevelPhase>>,
) {
    if !keyboard.just_pressed(KeyCode::F4) {
        return;
    }
    for entity in asteroid_q.iter() {
        if let Ok(mut e) = commands.get_entity(entity) {
            e.try_despawn();
        }
    }
    difficulty.level_complete = true;
    difficulty.active_spawners.clear();
    next.set(LevelPhase::Outro);
}

// ─── UI de l'outro ──────────────────────────────────────────────────

fn spawn_outro_ui(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    progress: &Res<GameProgress>,
) {
    let font = asset_server.load("fonts/PressStart2P-Regular.ttf");
    let name = level_name(progress.current_level);

    commands
        .spawn((
            (
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(30.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
                GlobalZIndex(90),
            ),
            OutroUI,
        ))
        .with_children(|parent| {
            parent.spawn((
                (
                    Text::new(name.to_uppercase()),
                    TextFont { font: font.clone(), font_size: 36.0, ..default() },
                    TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
                ),
                OutroUI,
            ));
            parent.spawn((
                (
                    Text::new("NIVEAU TERMINE"),
                    TextFont { font: font.clone(), font_size: 64.0, ..default() },
                    TextColor(Color::srgba(1.0, 0.85, 0.0, 1.0)),
                ),
                OutroUI,
            ));
            parent.spawn((
                Text::new("Appuyez sur Entree pour continuer"),
                TextFont { font, font_size: 24.0, ..default() },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
                OutroUI,
            ));
        });
}

/// Transition automatique LevelTransition → LevelSelect.
fn auto_start_next_level(mut next_state: ResMut<NextState<GameState>>) {
    next_state.set(GameState::LevelSelect);
}

/// Nettoyage en sortant de Playing. `LevelPhase` et ses Resources de phase
/// (`IntroData`, `OutroCountdownData`, `OutroData`) sont auto-cleanup
/// par le SubState. Les UI/sons de gameplay le sont via le marker
/// `GameplayEntity`. Reste seulement la popup de confirmation (qui peut
/// survivre à d'autres états → pas un `GameplayEntity`).
fn cleanup_playing(
    mut commands: Commands,
    confirm_ui_q: Query<Entity, With<ConfirmPopupUI>>,
) {
    commands.remove_resource::<ConfirmPopup>();
    for entity in confirm_ui_q.iter() {
        if let Ok(mut e) = commands.get_entity(entity) {
            e.try_despawn();
        }
    }
}

// ─── Popup de confirmation partagée ─────────────────────────────────

/// Spawne la popup de confirmation "Votre progression sera perdue."
pub(crate) fn spawn_confirm_popup(commands: &mut Commands, asset_server: &Res<AssetServer>) {
    let font = asset_server.load("fonts/PressStart2P-Regular.ttf");
    let ui_yellow = Color::srgba(1.0, 0.85, 0.0, 1.0);

    commands
        .spawn((
            (
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 1.0)),
                GlobalZIndex(200),
            ),
            ConfirmPopupUI,
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        padding: UiRect::all(Val::Px(4.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(ui_yellow),
                ))
                .with_children(|border| {
                    border
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                padding: UiRect::new(
                                    Val::Px(50.0),
                                    Val::Px(50.0),
                                    Val::Px(35.0),
                                    Val::Px(35.0),
                                ),
                                row_gap: Val::Px(25.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 1.0)),
                        ))
                        .with_children(|panel| {
                            panel.spawn((
                                Text::new("Votre progression sera perdue."),
                                TextFont { font: font.clone(), font_size: 22.0, ..default() },
                                TextColor(Color::WHITE),
                                ConfirmPopupUI,
                            ));
                            panel.spawn((
                                Text::new("Etes-vous sur de vouloir quitter ?"),
                                TextFont { font: font.clone(), font_size: 18.0, ..default() },
                                TextColor(ui_yellow),
                                ConfirmPopupUI,
                            ));
                            panel
                                .spawn((
                                    (
                                        Node {
                                            flex_direction: FlexDirection::Row,
                                            column_gap: Val::Px(80.0),
                                            margin: UiRect::top(Val::Px(10.0)),
                                            ..default()
                                        },
                                    ),
                                    ConfirmPopupUI,
                                ))
                                .with_children(|row| {
                                    row.spawn((
                                        Text::new("Non"),
                                        TextFont { font: font.clone(), font_size: 32.0, ..default() },
                                        TextColor(ui_yellow),
                                        ConfirmPopupUI,
                                        ConfirmOptionMarker(0),
                                    ));
                                    row.spawn((
                                        Text::new("Oui"),
                                        TextFont { font, font_size: 32.0, ..default() },
                                        TextColor(Color::srgba(0.6, 0.6, 0.6, 1.0)),
                                        ConfirmPopupUI,
                                        ConfirmOptionMarker(1),
                                    ));
                                });
                        });
                });
        });
}

/// Despawn tous les éléments de la popup de confirmation.
pub(crate) fn despawn_confirm_popup(
    commands: &mut Commands,
    confirm_ui_q: &Query<Entity, With<ConfirmPopupUI>>,
) {
    for entity in confirm_ui_q.iter() {
        if let Ok(mut e) = commands.get_entity(entity) {
            e.try_despawn();
        }
    }
}

// ─── Écran de fin (Credits) ─────────────────────────────────────────

#[derive(Component)]
struct CreditsUI;

fn setup_credits(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/PressStart2P-Regular.ttf");

    commands
        .spawn((
            (
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(40.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 1.0)),
            ),
            CreditsUI,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("MERCI D'AVOIR JOUE"),
                TextFont { font: font.clone(), font_size: 48.0, ..default() },
                TextColor(Color::WHITE),
                CreditsUI,
            ));
            parent.spawn((
                Text::new("Appuyez sur Entree"),
                TextFont { font, font_size: 24.0, ..default() },
                TextColor(Color::srgba(0.5, 0.5, 0.5, 1.0)),
                CreditsUI,
            ));
        });
}

fn handle_credits_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
        next_state.set(GameState::MainMenu);
    }
}

fn cleanup_credits(mut commands: Commands, ui_q: Query<Entity, With<CreditsUI>>) {
    for entity in ui_q.iter() {
        if let Ok(mut e) = commands.get_entity(entity) {
            e.try_despawn();
        }
    }
}
