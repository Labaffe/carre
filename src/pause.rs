//! Système de pause.
//!
//! Appuyer sur Échap pendant le Playing met le jeu en pause.
//! Un overlay s'affiche avec les options "Reprendre" et "Quitter".
//! Le temps de jeu est gelé tant que la pause est active.

use crate::state::GameState;
use crate::MusicMain;
use crate::boss::MusicBoss;
use bevy::app::AppExit;
use bevy::prelude::*;

pub struct PausePlugin;

impl Plugin for PausePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PauseState>()
            .add_systems(
                Update,
                handle_pause_input.run_if(in_state(GameState::Playing)),
            )
            .add_systems(OnExit(GameState::Playing), cleanup_pause);
    }
}

// ─── Ressource ──────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct PauseState {
    pub paused: bool,
    selected: usize,
}

// ─── Composants ─────────────────────────────────────────────────────

#[derive(Component)]
struct PauseUI;

#[derive(Component)]
struct PauseOption {
    action: PauseAction,
}

#[derive(Clone, PartialEq)]
enum PauseAction {
    Resume,
    MainMenu,
    Quit,
}

// ─── Systèmes ───────────────────────────────────────────────────────

fn handle_pause_input(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut pause: ResMut<PauseState>,
    mut time: ResMut<Time<Virtual>>,
    mut exit: EventWriter<AppExit>,
    mut next_state: ResMut<NextState<GameState>>,
    pause_ui_q: Query<Entity, With<PauseUI>>,
    mut text_q: Query<(&mut Text, &PauseOption)>,
    asset_server: Res<AssetServer>,
    music_q: Query<&AudioSink, With<MusicMain>>,
    boss_music_q: Query<&AudioSink, With<MusicBoss>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        if pause.paused {
            // Reprendre la musique
            for sink in music_q.iter() {
                sink.play();
            }
            for sink in boss_music_q.iter() {
                sink.play();
            }
            // Reprendre
            unpause(&mut commands, &mut pause, &mut time, &pause_ui_q);
        } else {
            // Mettre en pause
            pause.paused = true;
            pause.selected = 0;
            time.pause();
            // Mettre la musique en pause
            for sink in music_q.iter() {
                sink.pause();
            }
            for sink in boss_music_q.iter() {
                sink.pause();
            }
            // Son de pause
            commands.spawn(AudioBundle {
                source: asset_server.load("audio/pause.ogg"),
                settings: PlaybackSettings::ONCE,
            });
            spawn_pause_ui(&mut commands, &asset_server);
        }
        return;
    }

    if !pause.paused {
        return;
    }

    // Navigation haut/bas
    if keyboard.just_pressed(KeyCode::ArrowUp) || keyboard.just_pressed(KeyCode::KeyW) {
        if pause.selected > 0 {
            pause.selected -= 1;
        }
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) || keyboard.just_pressed(KeyCode::KeyS) {
        if pause.selected < 2 {
            pause.selected += 1;
        }
    }

    // Mise à jour des couleurs des options
    for (mut text, option) in text_q.iter_mut() {
        let is_selected = (option.action == PauseAction::Resume && pause.selected == 0)
            || (option.action == PauseAction::MainMenu && pause.selected == 1)
            || (option.action == PauseAction::Quit && pause.selected == 2);

        for section in text.sections.iter_mut() {
            if is_selected {
                section.style.color = Color::rgba(1.0, 0.85, 0.0, 1.0);
            } else {
                section.style.color = Color::rgba(0.6, 0.6, 0.6, 1.0);
            }
        }
    }

    // Validation
    if keyboard.just_pressed(KeyCode::Enter)
        || keyboard.just_pressed(KeyCode::Space)
        || mouse.just_pressed(MouseButton::Left)
    {
        match pause.selected {
            0 => {
                // Reprendre la musique
                for sink in music_q.iter() {
                    sink.play();
                }
                for sink in boss_music_q.iter() {
                    sink.play();
                }
                // Reprendre
                unpause(&mut commands, &mut pause, &mut time, &pause_ui_q);
            }
            1 => {
                // Retour au menu principal
                unpause(&mut commands, &mut pause, &mut time, &pause_ui_q);
                next_state.set(GameState::MainMenu);
            }
            2 => {
                // Quitter le jeu
                exit.send(AppExit);
            }
            _ => {}
        }
    }
}

fn unpause(
    commands: &mut Commands,
    pause: &mut ResMut<PauseState>,
    time: &mut ResMut<Time<Virtual>>,
    pause_ui_q: &Query<Entity, With<PauseUI>>,
) {
    pause.paused = false;
    time.unpause();
    for entity in pause_ui_q.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn spawn_pause_ui(commands: &mut Commands, asset_server: &Res<AssetServer>) {
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
                background_color: Color::rgba(0.0, 0.0, 0.0, 0.7).into(),
                z_index: ZIndex::Global(100),
                ..default()
            },
            PauseUI,
        ))
        .with_children(|parent| {
            // Titre PAUSE
            parent.spawn(TextBundle::from_section(
                "PAUSE",
                TextStyle {
                    font: font.clone(),
                    font_size: 64.0,
                    color: Color::WHITE,
                },
            ));

            // Option : Reprendre (sélectionnée par défaut → jaune)
            parent.spawn((
                TextBundle::from_section(
                    "Reprendre",
                    TextStyle {
                        font: font.clone(),
                        font_size: 36.0,
                        color: Color::rgba(1.0, 0.85, 0.0, 1.0),
                    },
                ),
                PauseOption {
                    action: PauseAction::Resume,
                },
            ));

            // Option : Menu principal
            parent.spawn((
                TextBundle::from_section(
                    "Menu principal",
                    TextStyle {
                        font: font.clone(),
                        font_size: 36.0,
                        color: Color::rgba(0.6, 0.6, 0.6, 1.0),
                    },
                ),
                PauseOption {
                    action: PauseAction::MainMenu,
                },
            ));

            // Option : Quitter
            parent.spawn((
                TextBundle::from_section(
                    "Quitter",
                    TextStyle {
                        font: font.clone(),
                        font_size: 36.0,
                        color: Color::rgba(0.6, 0.6, 0.6, 1.0),
                    },
                ),
                PauseOption {
                    action: PauseAction::Quit,
                },
            ));
        });
}

/// Nettoyage de la pause quand on quitte l'état Playing (ex: game over).
fn cleanup_pause(
    mut commands: Commands,
    mut pause: ResMut<PauseState>,
    mut time: ResMut<Time<Virtual>>,
    pause_ui_q: Query<Entity, With<PauseUI>>,
) {
    if pause.paused {
        pause.paused = false;
        time.unpause();
    }
    for entity in pause_ui_q.iter() {
        commands.entity(entity).despawn_recursive();
    }
}
