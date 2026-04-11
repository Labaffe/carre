//! Progression du jeu et outro de niveau.
//!
//! Le `GameProgress` suit le niveau courant. Quand le dernier boss meurt,
//! un countdown de 3s démarre, puis l'outro freeze le jeu et affiche
//! un écran de victoire avec la musique `stage_clear.ogg`.
//!
//! Le freeze utilise `PauseState.outro_active` : tous les systèmes gatés
//! par `not_paused()` s'arrêtent, mais le temps réel continue pour
//! animer l'outro.
//!
//! ## Flow
//! ```text
//! Boss meurt → 3s countdown → outro (freeze + musique + texte)
//!   → Entrée → LevelTransition → Playing (niveau suivant)
//!                ou MainMenu (dernier niveau terminé)
//! ```

use crate::asteroid::Asteroid;
use crate::boss::{BossMarker, MusicBoss};
use crate::difficulty::Difficulty;
use crate::enemy::{Enemy, EnemyState};
use crate::pause::PauseState;
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
                    detect_level_complete,
                    debug_skip_to_outro,
                    level_outro_animate,
                    level_outro_input,
                )
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(OnExit(GameState::Playing), cleanup_outro)
            .add_systems(
                OnEnter(GameState::LevelTransition),
                auto_start_next_level,
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

/// Countdown avant le déclenchement de l'outro (3s après la mort du boss).
#[derive(Resource)]
struct OutroCountdown(Timer);

/// Active pendant la séquence d'outro.
#[derive(Resource)]
struct LevelOutro {
    elapsed: f32,
    music_spawned: bool,
}

// ─── Composants ─────────────────────────────────────────────────────

/// Marqueur pour tous les éléments UI de l'outro (cleanup).
#[derive(Component)]
struct OutroUI;

/// Marqueur pour la musique de l'outro.
#[derive(Component)]
pub struct MusicOutro;

// ─── Constantes ─────────────────────────────────────────────────────

/// Délai entre la mort du boss et le début de l'outro (secondes).
const OUTRO_COUNTDOWN: f32 = 3.0;
/// Délai minimum avant d'accepter l'input pour continuer (secondes).
const OUTRO_INPUT_DELAY: f32 = 3.0;

// ─── Systèmes ───────────────────────────────────────────────────────

/// Lance la séquence d'outro : freeze le jeu, coupe les musiques,
/// stoppe le background, affiche l'UI.
fn start_outro(
    commands: &mut Commands,
    pause: &mut ResMut<PauseState>,
    difficulty: &mut ResMut<Difficulty>,
    asset_server: &Res<AssetServer>,
    music_q: &Query<Entity, With<MusicMain>>,
    boss_music_q: &Query<Entity, With<MusicBoss>>,
) {
    // Freeze le jeu via le flag outro (pas d'appel à time.pause(),
    // le temps réel continue pour animer l'outro)
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

    commands.remove_resource::<OutroCountdown>();
    commands.insert_resource(LevelOutro {
        elapsed: 0.0,
        music_spawned: false,
    });

    spawn_outro_ui(commands, asset_server);
}

/// Détecte quand tous les boss sont morts (entités despawnées) et lance
/// le countdown de 3s avant l'outro.
fn detect_level_complete(
    mut commands: Commands,
    time: Res<Time>,
    mut difficulty: ResMut<Difficulty>,
    boss_q: Query<&Enemy, With<BossMarker>>,
    countdown: Option<ResMut<OutroCountdown>>,
    outro: Option<Res<LevelOutro>>,
    mut pause: ResMut<PauseState>,
    asset_server: Res<AssetServer>,
    music_q: Query<Entity, With<MusicMain>>,
    boss_music_q: Query<Entity, With<MusicBoss>>,
) {
    // Déjà en outro → rien à faire
    if outro.is_some() {
        return;
    }

    // Le boss doit avoir été spawné ET toutes les entités boss doivent
    // avoir disparu (fin de l'animation de mort + despawn)
    if !difficulty.boss_spawned {
        return;
    }
    if !boss_q.is_empty() {
        return;
    }

    if let Some(mut cd) = countdown {
        cd.0.tick(time.delta());
        if cd.0.finished() {
            start_outro(
                &mut commands,
                &mut pause,
                &mut difficulty,
                &asset_server,
                &music_q,
                &boss_music_q,
            );
        }
    } else {
        // Premier frame de détection → démarrer le countdown
        commands.insert_resource(OutroCountdown(Timer::from_seconds(
            OUTRO_COUNTDOWN,
            TimerMode::Once,
        )));
    }
}

/// Anime l'écran d'outro : musique.
fn level_outro_animate(
    mut commands: Commands,
    time: Res<Time>,
    outro: Option<ResMut<LevelOutro>>,
    asset_server: Res<AssetServer>,
) {
    let Some(mut outro) = outro else { return };
    outro.elapsed += time.delta_seconds();

    // Lancer la musique une seule fois
    if !outro.music_spawned {
        outro.music_spawned = true;
        commands.spawn((
            AudioBundle {
                source: asset_server.load("audio/stage_clear.ogg"),
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
    outro: Option<Res<LevelOutro>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut pause: ResMut<PauseState>,
    mut progress: ResMut<GameProgress>,
    music_q: Query<Entity, With<MusicOutro>>,
) {
    let Some(outro) = outro else { return };

    // Attendre avant d'accepter l'input
    if outro.elapsed < OUTRO_INPUT_DELAY {
        return;
    }

    if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
        // Nettoyage
        for entity in music_q.iter() {
            if let Some(e) = commands.get_entity(entity) {
                e.despawn_recursive();
            }
        }
        pause.outro_active = false;
        commands.remove_resource::<LevelOutro>();

        if progress.current_level < progress.total_levels {
            // Niveau suivant
            progress.current_level += 1;
            next_state.set(GameState::LevelTransition);
        } else {
            // Jeu terminé → retour au menu
            next_state.set(GameState::MainMenu);
        }
    }
}

// ─── F4 : skip direct à l'outro ─────────────────────────────────────

/// F4 : tue tous les ennemis et déclenche l'outro immédiatement.
fn debug_skip_to_outro(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    outro: Option<Res<LevelOutro>>,
    mut pause: ResMut<PauseState>,
    mut difficulty: ResMut<Difficulty>,
    asset_server: Res<AssetServer>,
    enemy_q: Query<(Entity, &Enemy)>,
    asteroid_q: Query<Entity, With<Asteroid>>,
    music_q: Query<Entity, With<MusicMain>>,
    boss_music_q: Query<Entity, With<MusicBoss>>,
) {
    if !keyboard.just_pressed(KeyCode::F4) {
        return;
    }
    if outro.is_some() {
        return;
    }

    // Tuer tous les ennemis
    for (entity, enemy) in enemy_q.iter() {
        if matches!(enemy.state, EnemyState::Dying | EnemyState::Dead) {
            continue;
        }
        // Despawn direct (pas d'animation de mort)
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

    // Marquer le boss comme spawné pour que detect_level_complete se déclenche
    difficulty.boss_spawned = true;
    difficulty.active_spawners.clear();

    // Lancer l'outro immédiatement (sans countdown)
    start_outro(
        &mut commands,
        &mut pause,
        &mut difficulty,
        &asset_server,
        &music_q,
        &boss_music_q,
    );
}

// ─── UI de l'outro ──────────────────────────────────────────────────

fn spawn_outro_ui(commands: &mut Commands, asset_server: &Res<AssetServer>) {
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

/// Transition automatique LevelTransition → Playing (niveau suivant).
fn auto_start_next_level(mut next_state: ResMut<NextState<GameState>>) {
    next_state.set(GameState::Playing);
}

/// Nettoyage de l'outro en sortant de Playing.
fn cleanup_outro(
    mut commands: Commands,
    mut pause: ResMut<PauseState>,
    outro_ui_q: Query<Entity, With<OutroUI>>,
    music_q: Query<Entity, With<MusicOutro>>,
) {
    pause.outro_active = false;
    commands.remove_resource::<LevelOutro>();
    commands.remove_resource::<OutroCountdown>();
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
}
