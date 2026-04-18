//! Progression du jeu — machine à état du niveau.
//!
//! Chaque niveau suit le flow :
//! ```text
//! Intro (optionnelle) → Running (LevelSteps) → OutroCountdown → Outro
//! ```
//!
//! - **Intro** : le vaisseau monte depuis le bas de l'écran + son.
//!   Le gameplay est gelé via `PauseState.intro_active`.
//! - **Running** : les LevelSteps s'exécutent. Le niveau ne se termine
//!   PAS quand toutes les étapes sont jouées — il faut un événement
//!   explicite (`MarkLevelComplete` ou mort du dernier boss).
//! - **OutroCountdown** : 3s d'attente.
//! - **Outro** : freeze le jeu, musique `stage_clear.ogg`, écran de victoire.

use std::collections::HashSet;

use crate::asteroid::Asteroid;
use crate::boss::{BossMarker, MusicBoss};
use crate::difficulty::Difficulty;
use crate::enemy::{Enemy, EnemyState};
use crate::level::{LevelConfig, level_name};
use crate::levels::ScrollDirection;
use crate::pause::PauseState;
use crate::player::Player;
use crate::state::GameState;
use crate::MusicMain;
use bevy::prelude::*;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameProgress>()
            .add_systems(
                Update,
                (
                    level_phase_system,
                    skip_intro_input,
                    detect_boss_death,
                    detect_level_complete,
                    debug_skip_to_outro,
                    level_outro_animate,
                    level_outro_input,
                )
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(OnExit(GameState::Playing), cleanup_playing)
            .add_systems(
                OnEnter(GameState::LevelTransition),
                auto_start_next_level,
            )
            .add_systems(OnEnter(GameState::Credits), setup_credits)
            .add_systems(OnExit(GameState::Credits), cleanup_credits)
            .add_systems(
                Update,
                handle_credits_input.run_if(in_state(GameState::Credits)),
            );
    }
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
            total_levels: 2,
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
//  Machine à état du niveau : LevelPhase
// ═══════════════════════════════════════════════════════════════════════

/// Machine à état du niveau en cours.
///
/// ```text
/// Intro (optionnel) → Running → OutroCountdown → Outro
/// ```
#[derive(Resource)]
pub struct LevelPhase {
    pub phase: LevelPhaseKind,
}

#[derive(Debug, Clone)]
pub enum LevelPhaseKind {
    /// Animation d'entrée du vaisseau (optionnelle).
    /// L'intro se termine quand le son ET l'animation sont finis.
    Intro {
        elapsed: f32,
        /// Durée de l'animation du vaisseau (= durée du son).
        duration: f32,
        sound: &'static str,
        sound_played: bool,
        /// Le son d'intro a fini de jouer (entité IntroSound despawnée).
        sound_finished: bool,
        /// Position de départ (hors écran).
        start_pos: Vec2,
        /// Position cible (en jeu).
        target_pos: Vec2,
        /// Ratio pour calculer la position cible (ex: -0.5 → 50% du half-screen).
        spawn_ratio: f32,
        initialized: bool,
    },
    /// Niveau en cours : les LevelSteps s'exécutent.
    Running,
    /// Countdown avant l'outro (3s après level_complete).
    OutroCountdown { timer: Timer },
    /// Séquence d'outro (victoire).
    Outro { elapsed: f32, music_spawned: bool },
}

// ─── Composants ─────────────────────────────────────────────────────

/// Marqueur pour tous les éléments UI de l'outro (cleanup).
#[derive(Component)]
struct OutroUI;

/// Marqueur pour le son d'intro (landing.ogg). Despawné automatiquement
/// par Bevy quand la lecture est terminée (PlaybackSettings::DESPAWN).
#[derive(Component)]
pub struct IntroSound;

/// Marqueur pour la musique de l'outro.
#[derive(Component)]
pub struct MusicOutro;

// ─── Constantes ─────────────────────────────────────────────────────

/// Délai entre level_complete et le début de l'outro (secondes).
const OUTRO_COUNTDOWN: f32 = 3.0;
/// Délai minimum avant d'accepter l'input pour continuer (secondes).
const OUTRO_INPUT_DELAY: f32 = 3.0;

// ─── Configuration d'intro par niveau ───────────────────────────────

/// Configuration d'une intro de niveau.
pub struct IntroConfig {
    /// Durée de l'animation d'entrée du vaisseau (= durée du son).
    pub duration: f32,
    /// Son joué pendant l'intro.
    pub sound: &'static str,
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
            sound: "audio/sfx/landing.ogg",
            spawn_ratio: -0.5,
        },
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Systèmes
// ═══════════════════════════════════════════════════════════════════════

/// Système principal de la machine à état du niveau.
/// Gère Intro (animation vaisseau) et OutroCountdown (tick timer).
fn level_phase_system(
    mut commands: Commands,
    time: Res<Time>,
    mut pause: ResMut<PauseState>,
    asset_server: Res<AssetServer>,
    mut player_q: Query<&mut Transform, With<Player>>,
    windows: Query<&Window>,
    level_phase: Option<ResMut<LevelPhase>>,
    intro_sound_q: Query<Entity, With<IntroSound>>,
    config: Res<LevelConfig>,
) {
    let Some(mut level_phase) = level_phase else { return };

    match &mut level_phase.phase {
        LevelPhaseKind::Intro {
            elapsed,
            duration,
            sound,
            sound_played,
            sound_finished,
            start_pos,
            target_pos,
            spawn_ratio,
            initialized,
        } => {
            let window = windows.single();
            let half_w = window.width() / 2.0;
            let half_h = window.height() / 2.0;

            // Premier frame : initialiser les positions selon la direction du scroll
            if !*initialized {
                let (start, target) = match config.scroll_direction {
                    ScrollDirection::Down => (
                        Vec2::new(0.0, -half_h - 150.0),
                        Vec2::new(0.0, half_h * *spawn_ratio),
                    ),
                    ScrollDirection::Up => (
                        Vec2::new(0.0, half_h + 150.0),
                        Vec2::new(0.0, -(half_h * *spawn_ratio)),
                    ),
                    ScrollDirection::Left => (
                        Vec2::new(-half_w - 150.0, 0.0),
                        Vec2::new(half_w * *spawn_ratio, 0.0),
                    ),
                    ScrollDirection::Right => (
                        Vec2::new(half_w + 150.0, 0.0),
                        Vec2::new(-(half_w * *spawn_ratio), 0.0),
                    ),
                };
                *start_pos = start;
                *target_pos = target;
                *initialized = true;
                pause.intro_active = true;

                // Rotation du vaisseau selon la direction d'entrée
                let ship_angle = match config.scroll_direction {
                    ScrollDirection::Down => 0.0,                                    // pointe vers le haut
                    ScrollDirection::Up => std::f32::consts::PI,                     // pointe vers le bas
                    ScrollDirection::Left => -std::f32::consts::FRAC_PI_2,          // pointe vers la droite
                    ScrollDirection::Right => std::f32::consts::FRAC_PI_2,          // pointe vers la gauche
                };

                if let Ok(mut transform) = player_q.get_single_mut() {
                    transform.translation.x = start_pos.x;
                    transform.translation.y = start_pos.y;
                    transform.rotation = Quat::from_rotation_z(ship_angle);
                }
            }

            // Ne pas avancer l'intro pendant la pause
            if pause.paused {
                return;
            }

            // Jouer le son une seule fois (avec marqueur IntroSound)
            if !*sound_played {
                *sound_played = true;
                commands.spawn((
                    AudioBundle {
                        source: asset_server.load(*sound),
                        settings: PlaybackSettings::DESPAWN,
                    },
                    IntroSound,
                ));
            }

            // Détecter la fin du son (entité IntroSound despawnée par Bevy)
            if *sound_played && !*sound_finished && intro_sound_q.is_empty() {
                *sound_finished = true;
            }

            *elapsed += time.delta_seconds();
            let anim_t = (*elapsed / *duration).clamp(0.0, 1.0);

            // Ease-out quadratique
            let eased = 1.0 - (1.0 - anim_t).powi(2);

            if let Ok(mut transform) = player_q.get_single_mut() {
                let pos = *start_pos + (*target_pos - *start_pos) * eased;
                transform.translation.x = pos.x;
                transform.translation.y = pos.y;
            }

            // Intro terminée quand l'animation ET le son sont finis
            if anim_t >= 1.0 && *sound_finished {
                if let Ok(mut transform) = player_q.get_single_mut() {
                    transform.translation.x = target_pos.x;
                    transform.translation.y = target_pos.y;
                }
                pause.intro_active = false;
                level_phase.phase = LevelPhaseKind::Running;
            }
        }
        LevelPhaseKind::Running => {
            // Le LevelRunner tourne dans level.rs
        }
        LevelPhaseKind::OutroCountdown { timer } => {
            timer.tick(time.delta());
            // La transition vers Outro est gérée par detect_level_complete
        }
        LevelPhaseKind::Outro { .. } => {
            // Géré par level_outro_animate et level_outro_input
        }
    }
}

/// Skip l'intro avec Entrée ou clic gauche.
fn skip_intro_input(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut level_phase: Option<ResMut<LevelPhase>>,
    mut pause: ResMut<PauseState>,
    mut player_q: Query<&mut Transform, With<Player>>,
    intro_sound_q: Query<Entity, With<IntroSound>>,
    windows: Query<&Window>,
    config: Res<LevelConfig>,
) {
    if !keyboard.just_pressed(KeyCode::Enter)
        && !keyboard.just_pressed(KeyCode::Space)
        && !mouse.just_pressed(MouseButton::Left)
    {
        return;
    }

    let Some(ref mut level_phase) = level_phase else { return };
    if !matches!(level_phase.phase, LevelPhaseKind::Intro { .. }) {
        return;
    }
    if pause.paused {
        return;
    }

    do_skip_intro(&mut commands, level_phase, &mut pause, &mut player_q, &intro_sound_q, &windows, &config);
}

/// Skip l'intro : place le joueur à sa position cible, despawn le son, passe en Running.
pub(crate) fn do_skip_intro(
    commands: &mut Commands,
    level_phase: &mut ResMut<LevelPhase>,
    pause: &mut ResMut<PauseState>,
    player_q: &mut Query<&mut Transform, With<Player>>,
    intro_sound_q: &Query<Entity, With<IntroSound>>,
    windows: &Query<&Window>,
    config: &Res<LevelConfig>,
) {
    // Calculer la position cible à partir du ratio si l'intro n'a pas été initialisée
    let final_pos = if let LevelPhaseKind::Intro { target_pos, spawn_ratio, initialized, .. } = &level_phase.phase {
        if *initialized {
            *target_pos
        } else {
            let window = windows.single();
            let half_w = window.width() / 2.0;
            let half_h = window.height() / 2.0;
            match config.scroll_direction {
                ScrollDirection::Down => Vec2::new(0.0, half_h * *spawn_ratio),
                ScrollDirection::Up => Vec2::new(0.0, -(half_h * *spawn_ratio)),
                ScrollDirection::Left => Vec2::new(half_w * *spawn_ratio, 0.0),
                ScrollDirection::Right => Vec2::new(-(half_w * *spawn_ratio), 0.0),
            }
        }
    } else {
        return;
    };

    // Placer le joueur à sa position cible
    if let Ok(mut transform) = player_q.get_single_mut() {
        transform.translation.x = final_pos.x;
        transform.translation.y = final_pos.y;
    }

    // Despawn le son d'intro
    for entity in intro_sound_q.iter() {
        if let Some(e) = commands.get_entity(entity) {
            e.despawn_recursive();
        }
    }

    // Passer en Running
    pause.intro_active = false;
    level_phase.phase = LevelPhaseKind::Running;
}

/// Détecte la fin de l'animation de mort du dernier boss et envoie
/// `MarkLevelComplete` via le pipeline du niveau.
/// Pour les niveaux sans boss, `MarkLevelComplete` dans la timeline fait le travail.
fn detect_boss_death(
    mut difficulty: ResMut<Difficulty>,
    boss_q: Query<&Enemy, With<BossMarker>>,
    mut level_events: EventWriter<crate::level::LevelActionEvent>,
) {
    // Marquer qu'on a vu un boss vivant (évite la race condition avec Commands différées).
    if !difficulty.boss_seen_alive && !boss_q.is_empty() {
        difficulty.boss_seen_alive = true;
    }

    // Le boss a été vu vivant, toutes les entités boss ont disparu (fin d'anim de mort),
    // et le niveau n'est pas encore marqué comme terminé.
    if difficulty.boss_seen_alive && boss_q.is_empty() && !difficulty.level_complete {
        level_events.send(crate::level::LevelActionEvent(vec![
            crate::level::Action::MarkLevelComplete,
        ]));
    }
}

/// Vérifie `level_complete` et fait avancer la machine à état :
/// Running → OutroCountdown → Outro.
fn detect_level_complete(
    mut commands: Commands,
    mut difficulty: ResMut<Difficulty>,
    mut pause: ResMut<PauseState>,
    asset_server: Res<AssetServer>,
    music_q: Query<Entity, With<MusicMain>>,
    boss_music_q: Query<Entity, With<MusicBoss>>,
    progress: Res<GameProgress>,
    mut level_phase: Option<ResMut<LevelPhase>>,
) {
    let Some(ref mut level_phase) = level_phase else { return };

    match &level_phase.phase {
        LevelPhaseKind::Running => {
            if !difficulty.level_complete {
                return;
            }
            // Running → OutroCountdown
            level_phase.phase = LevelPhaseKind::OutroCountdown {
                timer: Timer::from_seconds(OUTRO_COUNTDOWN, TimerMode::Once),
            };
        }
        LevelPhaseKind::OutroCountdown { timer } => {
            if timer.finished() {
                // OutroCountdown → Outro
                start_outro(
                    &mut commands,
                    &mut pause,
                    &mut difficulty,
                    &asset_server,
                    &music_q,
                    &boss_music_q,
                    &progress,
                    level_phase,
                );
            }
        }
        _ => {}
    }
}

/// Lance la séquence d'outro : freeze le jeu, coupe les musiques,
/// stoppe le background, affiche l'UI.
fn start_outro(
    commands: &mut Commands,
    pause: &mut ResMut<PauseState>,
    difficulty: &mut ResMut<Difficulty>,
    asset_server: &Res<AssetServer>,
    music_q: &Query<Entity, With<MusicMain>>,
    boss_music_q: &Query<Entity, With<MusicBoss>>,
    progress: &Res<GameProgress>,
    level_phase: &mut ResMut<LevelPhase>,
) {
    pause.outro_active = true;

    // Couper les musiques
    for entity in music_q.iter() {
        if let Some(e) = commands.get_entity(entity) {
            e.despawn_recursive();
        }
    }
    for entity in boss_music_q.iter() {
        if let Some(e) = commands.get_entity(entity) {
            e.despawn_recursive();
        }
    }

    // Stopper le background
    difficulty.bg_speed_override = Some(0.0);

    // Transition de phase → Outro
    level_phase.phase = LevelPhaseKind::Outro {
        elapsed: 0.0,
        music_spawned: false,
    };

    spawn_outro_ui(commands, asset_server, progress);
}

/// Anime l'écran d'outro : musique.
fn level_outro_animate(
    mut commands: Commands,
    time: Res<Time>,
    level_phase: Option<ResMut<LevelPhase>>,
    asset_server: Res<AssetServer>,
) {
    let Some(mut level_phase) = level_phase else { return };
    let LevelPhaseKind::Outro { elapsed, music_spawned } = &mut level_phase.phase else { return };

    *elapsed += time.delta_seconds();

    if !*music_spawned {
        *music_spawned = true;
        commands.spawn((
            AudioBundle {
                source: asset_server.load("audio/music/stage_clear.ogg"),
                settings: PlaybackSettings::ONCE,
            },
            MusicOutro,
        ));
    }
}

/// Gère l'input pendant l'outro (Entrée pour continuer).
fn level_outro_input(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    level_phase: Option<Res<LevelPhase>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut pause: ResMut<PauseState>,
    progress: Res<GameProgress>,
    play_mode: Option<Res<PlayMode>>,
    mut campaign: Option<ResMut<CampaignProgress>>,
    music_q: Query<Entity, With<MusicOutro>>,
) {
    let Some(ref level_phase) = level_phase else { return };
    let LevelPhaseKind::Outro { elapsed, .. } = &level_phase.phase else { return };

    if *elapsed < OUTRO_INPUT_DELAY {
        return;
    }

    if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
        for entity in music_q.iter() {
            if let Some(e) = commands.get_entity(entity) {
                e.despawn_recursive();
            }
        }
        pause.outro_active = false;

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
}

// ─── F4 : skip direct à l'outro ─────────────────────────────────────

/// F4 : tue tous les ennemis et déclenche l'outro immédiatement.
fn debug_skip_to_outro(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut pause: ResMut<PauseState>,
    mut difficulty: ResMut<Difficulty>,
    asset_server: Res<AssetServer>,
    enemy_q: Query<(Entity, &Enemy)>,
    asteroid_q: Query<Entity, With<Asteroid>>,
    music_q: Query<Entity, With<MusicMain>>,
    boss_music_q: Query<Entity, With<MusicBoss>>,
    progress: Res<GameProgress>,
    mut level_phase: Option<ResMut<LevelPhase>>,
) {
    if !keyboard.just_pressed(KeyCode::F4) {
        return;
    }
    let Some(ref mut level_phase) = level_phase else { return };
    if matches!(level_phase.phase, LevelPhaseKind::Outro { .. }) {
        return;
    }

    // Tuer tous les ennemis
    for (entity, enemy) in enemy_q.iter() {
        if matches!(enemy.state, EnemyState::Dying | EnemyState::Dead) {
            continue;
        }
        if let Some(e) = commands.get_entity(entity) {
            e.despawn_recursive();
        }
    }

    // Despawn tous les astéroïdes
    for entity in asteroid_q.iter() {
        if let Some(e) = commands.get_entity(entity) {
            e.despawn_recursive();
        }
    }

    // Marquer le niveau comme terminé
    difficulty.level_complete = true;
    difficulty.active_spawners.clear();

    // Désactiver l'intro si elle était en cours
    pause.intro_active = false;

    // Lancer l'outro immédiatement (sans countdown)
    start_outro(
        &mut commands,
        &mut pause,
        &mut difficulty,
        &asset_server,
        &music_q,
        &boss_music_q,
        &progress,
        level_phase,
    );
}

// ─── UI de l'outro ──────────────────────────────────────────────────

fn spawn_outro_ui(commands: &mut Commands, asset_server: &Res<AssetServer>, progress: &Res<GameProgress>) {
    let font = asset_server.load("fonts/PressStart2P-Regular.ttf");
    let name = level_name(progress.current_level);

    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(30.0),
                    ..default()
                },
                background_color: Color::rgba(0.0, 0.0, 0.0, 0.6).into(),
                z_index: ZIndex::Global(90),
                ..default()
            },
            OutroUI,
        ))
        .with_children(|parent| {
            // Nom du niveau
            parent.spawn((
                TextBundle::from_section(
                    name.to_uppercase(),
                    TextStyle {
                        font: font.clone(),
                        font_size: 36.0,
                        color: Color::rgba(1.0, 1.0, 1.0, 1.0),
                    },
                ),
                OutroUI,
            ));
            // Titre
            parent.spawn((
                TextBundle::from_section(
                    "NIVEAU TERMINE",
                    TextStyle {
                        font: font.clone(),
                        font_size: 64.0,
                        color: Color::rgba(1.0, 0.85, 0.0, 1.0),
                    },
                ),
                OutroUI,
            ));
            // Instruction
            parent.spawn((
                TextBundle::from_section(
                    "Appuyez sur Entree pour continuer",
                    TextStyle {
                        font,
                        font_size: 24.0,
                        color: Color::rgba(1.0, 1.0, 1.0, 1.0),
                    },
                ),
                OutroUI,
            ));
        });
}

/// Transition automatique LevelTransition → LevelSelect.
fn auto_start_next_level(mut next_state: ResMut<NextState<GameState>>) {
    next_state.set(GameState::LevelSelect);
}

/// Nettoyage en sortant de Playing (intro, outro, popups).
fn cleanup_playing(
    mut commands: Commands,
    mut pause: ResMut<PauseState>,
    outro_ui_q: Query<Entity, With<OutroUI>>,
    music_q: Query<Entity, With<MusicOutro>>,
    intro_sound_q: Query<Entity, With<IntroSound>>,
    confirm_ui_q: Query<Entity, With<ConfirmPopupUI>>,
) {
    pause.intro_active = false;
    pause.outro_active = false;
    commands.remove_resource::<LevelPhase>();
    commands.remove_resource::<ConfirmPopup>();
    for entity in intro_sound_q.iter() {
        if let Some(e) = commands.get_entity(entity) {
            e.despawn_recursive();
        }
    }
    for entity in outro_ui_q.iter() {
        if let Some(e) = commands.get_entity(entity) {
            e.despawn_recursive();
        }
    }
    for entity in music_q.iter() {
        if let Some(e) = commands.get_entity(entity) {
            e.despawn_recursive();
        }
    }
    for entity in confirm_ui_q.iter() {
        if let Some(e) = commands.get_entity(entity) {
            e.despawn_recursive();
        }
    }
}

// ─── Popup de confirmation partagée ─────────────────────────────────

/// Spawne la popup de confirmation "Votre progression sera perdue."
pub(crate) fn spawn_confirm_popup(commands: &mut Commands, asset_server: &Res<AssetServer>) {
    let font = asset_server.load("fonts/PressStart2P-Regular.ttf");
    let ui_yellow = Color::rgba(1.0, 0.85, 0.0, 1.0);

    // Fond opaque plein écran
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                background_color: Color::rgba(0.0, 0.0, 0.0, 1.0).into(),
                z_index: ZIndex::Global(200),
                ..default()
            },
            ConfirmPopupUI,
        ))
        .with_children(|overlay| {
            // Bordure jaune (padding = épaisseur du bord)
            overlay
                .spawn(NodeBundle {
                    style: Style {
                        padding: UiRect::all(Val::Px(4.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    background_color: ui_yellow.into(),
                    ..default()
                })
                .with_children(|border| {
                    // Panneau noir intérieur
                    border
                        .spawn(NodeBundle {
                            style: Style {
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
                            background_color: Color::rgba(0.0, 0.0, 0.0, 1.0).into(),
                            ..default()
                        })
                        .with_children(|panel| {
                            // Question
                            panel.spawn((
                                TextBundle::from_section(
                                    "Votre progression sera perdue.",
                                    TextStyle {
                                        font: font.clone(),
                                        font_size: 22.0,
                                        color: Color::WHITE,
                                    },
                                ),
                                ConfirmPopupUI,
                            ));

                            // Avertissement
                            panel.spawn((
                                TextBundle::from_section(
                                    "Etes-vous sur de vouloir quitter ?",
                                    TextStyle {
                                        font: font.clone(),
                                        font_size: 18.0,
                                        color: ui_yellow,
                                    },
                                ),
                                ConfirmPopupUI,
                            ));

                            // Options côte à côte
                            panel
                                .spawn((
                                    NodeBundle {
                                        style: Style {
                                            flex_direction: FlexDirection::Row,
                                            column_gap: Val::Px(80.0),
                                            margin: UiRect::top(Val::Px(10.0)),
                                            ..default()
                                        },
                                        ..default()
                                    },
                                    ConfirmPopupUI,
                                ))
                                .with_children(|row| {
                                    row.spawn((
                                        TextBundle::from_section(
                                            "Non",
                                            TextStyle {
                                                font: font.clone(),
                                                font_size: 32.0,
                                                color: ui_yellow,
                                            },
                                        ),
                                        ConfirmPopupUI,
                                        ConfirmOptionMarker(0),
                                    ));
                                    row.spawn((
                                        TextBundle::from_section(
                                            "Oui",
                                            TextStyle {
                                                font,
                                                font_size: 32.0,
                                                color: Color::rgba(0.6, 0.6, 0.6, 1.0),
                                            },
                                        ),
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
        if let Some(e) = commands.get_entity(entity) {
            e.despawn_recursive();
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
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(40.0),
                    ..default()
                },
                background_color: Color::rgba(0.0, 0.0, 0.0, 1.0).into(),
                ..default()
            },
            CreditsUI,
        ))
        .with_children(|parent| {
            parent.spawn((
                TextBundle::from_section(
                    "MERCI D'AVOIR JOUE",
                    TextStyle {
                        font: font.clone(),
                        font_size: 48.0,
                        color: Color::WHITE,
                    },
                ),
                CreditsUI,
            ));
            parent.spawn((
                TextBundle::from_section(
                    "Appuyez sur Entree",
                    TextStyle {
                        font,
                        font_size: 24.0,
                        color: Color::rgba(0.5, 0.5, 0.5, 1.0),
                    },
                ),
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
        if let Some(e) = commands.get_entity(entity) {
            e.despawn_recursive();
        }
    }
}
