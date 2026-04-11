//! Écran de sélection de niveaux.
//!
//! Utilisé en mode Campagne et Primes. Le titre et le comportement
//! (Escape, niveaux grisés) dépendent du `PlayMode` actif.

use crate::game::{CampaignProgress, GameProgress, PlayMode};
use crate::level::level_name;
use crate::state::GameState;
use bevy::prelude::*;

pub struct LevelSelectPlugin;

impl Plugin for LevelSelectPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::LevelSelect), setup_level_select)
            .add_systems(OnExit(GameState::LevelSelect), cleanup_level_select)
            .add_systems(
                Update,
                (animate_level_select, handle_level_select_input)
                    .run_if(in_state(GameState::LevelSelect)),
            );
    }
}

// ─── Composants ─────────────────────────────────────────────────────

#[derive(Component)]
struct LevelSelectUI;

#[derive(Component)]
struct LevelSelectOption(usize); // index 0-based du niveau

// ─── Ressource ──────────────────────────────────────────────────────

#[derive(Resource)]
struct LevelSelectState {
    selected: usize,
}

// ─── Setup ──────────────────────────────────────────────────────────

fn setup_level_select(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    play_mode: Option<Res<PlayMode>>,
    progress: Res<GameProgress>,
    campaign: Option<Res<CampaignProgress>>,
) {
    let font = asset_server.load("fonts/PressStart2P-Regular.ttf");
    let mode = play_mode.map(|m| *m).unwrap_or(PlayMode::Primes);

    let title = match mode {
        PlayMode::Campaign => "CAMPAGNE",
        PlayMode::Primes => "PRIMES",
    };

    let footer = match mode {
        PlayMode::Campaign => "Entree pour jouer",
        PlayMode::Primes => "Entree pour jouer | Echap pour revenir",
    };

    // Niveaux complétés (uniquement en Campagne)
    let completed: std::collections::HashSet<usize> = match mode {
        PlayMode::Campaign => campaign.map(|c| c.completed.clone()).unwrap_or_default(),
        PlayMode::Primes => std::collections::HashSet::new(), // libre en Primes
    };

    // Trouver le premier niveau non-complété pour le curseur
    let first_available = (0..progress.total_levels)
        .find(|i| !completed.contains(&(i + 1)))
        .unwrap_or(0);

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
                background_color: Color::rgba(0.0, 0.0, 0.0, 1.0).into(),
                ..default()
            },
            LevelSelectUI,
        ))
        .with_children(|parent| {
            // Titre
            parent.spawn((
                TextBundle::from_section(
                    title,
                    TextStyle {
                        font: font.clone(),
                        font_size: 48.0,
                        color: Color::WHITE,
                    },
                ),
                LevelSelectUI,
            ));

            // Liste des niveaux
            for i in 0..progress.total_levels {
                let level_num = i + 1;
                let is_completed = completed.contains(&level_num);
                let is_selected = i == first_available;

                let color = if is_completed {
                    Color::rgba(0.3, 0.3, 0.3, 1.0) // grisé
                } else if is_selected {
                    Color::rgba(1.0, 0.85, 0.0, 1.0) // sélectionné
                } else {
                    Color::rgba(0.6, 0.6, 0.6, 1.0)
                };

                let label = if is_completed {
                    format!("{}. {} [OK]", level_num, level_name(level_num))
                } else {
                    format!("{}. {}", level_num, level_name(level_num))
                };

                parent.spawn((
                    TextBundle::from_section(
                        label,
                        TextStyle {
                            font: font.clone(),
                            font_size: 32.0,
                            color,
                        },
                    ),
                    LevelSelectUI,
                    LevelSelectOption(i),
                ));
            }

            // Footer
            parent.spawn((
                TextBundle::from_section(
                    footer,
                    TextStyle {
                        font,
                        font_size: 18.0,
                        color: Color::rgba(0.5, 0.5, 0.5, 1.0),
                    },
                ),
                LevelSelectUI,
            ));
        });

    commands.insert_resource(LevelSelectState {
        selected: first_available,
    });
}

// ─── Animation ──────────────────────────────────────────────────────

fn animate_level_select(
    state: Option<Res<LevelSelectState>>,
    play_mode: Option<Res<PlayMode>>,
    campaign: Option<Res<CampaignProgress>>,
    mut text_q: Query<(&mut Text, &LevelSelectOption)>,
) {
    let Some(state) = state else { return };
    let mode = play_mode.map(|m| *m).unwrap_or(PlayMode::Primes);

    let completed: std::collections::HashSet<usize> = match mode {
        PlayMode::Campaign => campaign.map(|c| c.completed.clone()).unwrap_or_default(),
        PlayMode::Primes => std::collections::HashSet::new(),
    };

    for (mut text, opt) in text_q.iter_mut() {
        let level_num = opt.0 + 1;
        let is_completed = completed.contains(&level_num);
        let is_selected = opt.0 == state.selected;

        for section in text.sections.iter_mut() {
            if is_completed {
                section.style.color = Color::rgba(0.3, 0.3, 0.3, 1.0);
            } else if is_selected {
                section.style.color = Color::rgba(1.0, 0.85, 0.0, 1.0);
            } else {
                section.style.color = Color::rgba(0.6, 0.6, 0.6, 1.0);
            }
        }
    }
}

// ─── Input ──────────────────────────────────────────────────────────

fn handle_level_select_input(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut state: Option<ResMut<LevelSelectState>>,
    play_mode: Option<Res<PlayMode>>,
    campaign: Option<Res<CampaignProgress>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut progress: ResMut<GameProgress>,
) {
    let Some(ref mut state) = state else { return };
    let mode = play_mode.map(|m| *m).unwrap_or(PlayMode::Primes);

    let completed: std::collections::HashSet<usize> = match mode {
        PlayMode::Campaign => campaign.map(|c| c.completed.clone()).unwrap_or_default(),
        PlayMode::Primes => std::collections::HashSet::new(),
    };

    // Navigation haut/bas
    if keyboard.just_pressed(KeyCode::ArrowUp) || keyboard.just_pressed(KeyCode::KeyW) {
        if state.selected > 0 {
            state.selected -= 1;
        }
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) || keyboard.just_pressed(KeyCode::KeyS) {
        if state.selected < progress.total_levels - 1 {
            state.selected += 1;
        }
    }

    // Lancer un niveau (seulement si non-complété)
    if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
        let level_num = state.selected + 1;
        if !completed.contains(&level_num) {
            progress.current_level = level_num;
            next_state.set(GameState::Playing);
        }
    }

    // Escape — retour au menu (Primes uniquement)
    if keyboard.just_pressed(KeyCode::Escape) {
        if mode == PlayMode::Primes {
            commands.remove_resource::<PlayMode>();
            next_state.set(GameState::MainMenu);
        }
        // En Campaign, Escape ne fait rien
    }
}

// ─── Cleanup ────────────────────────────────────────────────────────

fn cleanup_level_select(
    mut commands: Commands,
    ui_q: Query<Entity, With<LevelSelectUI>>,
) {
    commands.remove_resource::<LevelSelectState>();
    for entity in ui_q.iter() {
        if let Some(e) = commands.get_entity(entity) {
            e.despawn_recursive();
        }
    }
}
