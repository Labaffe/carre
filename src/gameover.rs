//! Écran de game over.
//!
//! Au déclenchement : supprime astéroïdes/missiles, cache le background (écran noir).
//! Après 1.5s de délai : lance la musique "you_died" et anime le texte
//! (fondu + zoom de 0.3 à 1.0 sur 6 secondes). Police : Optimus Princeps.
//! Appuyer sur R : nettoie l'UI, réaffiche le background, respawn le joueur.

use crate::game::{
    CampaignProgress, ConfirmOptionMarker, ConfirmPopup, ConfirmPopupUI, PlayMode,
    despawn_confirm_popup, spawn_confirm_popup,
};
use crate::state::GameState;
use crate::{MusicGameOver, MusicMain};
use bevy::prelude::*;

pub struct GameOverPlugin;

impl Plugin for GameOverPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(GameState::GameOver),
            (cleanup_playing_entities, setup_gameover_ui, stop_main_music),
        )
        .add_systems(
            OnExit(GameState::GameOver),
            (cleanup_gameover_ui, cleanup_gameover_anim),
        )
        .add_systems(
            Update,
            (animate_gameover, handle_restart).run_if(in_state(GameState::GameOver)),
        );
    }
}

// --- Composants ---

#[derive(Component)]
struct GameOverUI;

#[derive(Component)]
struct GameOverText;

#[derive(Component)]
struct GameOverBackground;

// --- Ressource d'animation ---

#[derive(Resource)]
struct GameOverAnim {
    elapsed: f32,
    music_spawned: bool,
}

// --- Setup ---

/// Les entités de jeu (joueur, astéroïdes, missiles, etc.) sont nettoyées
/// par `cleanup_playing` dans main.rs via OnExit(Playing).
/// Cette fonction ne fait plus rien car le nettoyage est centralisé.
fn cleanup_playing_entities() {}

fn setup_gameover_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/optimus_princeps.ttf");
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(20.0),
                    ..default()
                },
                // fond entièrement noir au départ
                background_color: Color::rgba(0.0, 0.0, 0.0, 1.0).into(),
                ..default()
            },
            GameOverUI,
            GameOverBackground,
        ))
        .with_children(|parent| {
            // texte invisible au départ (alpha = 0, scale réduit via Transform)
            parent.spawn((
                TextBundle::from_section(
                    "VOUS ETES MORT",
                    TextStyle {
                        font: font.clone(),
                        font_size: 90.0,
                        color: Color::rgba(1.0, 0.0, 0.0, 0.0),
                    },
                ),
                GameOverText,
            ));
            parent.spawn((
                TextBundle::from_section(
                    "R pour rejouer | Echap pour quitter",
                    TextStyle {
                        font: font.clone(),
                        font_size: 28.0,
                        color: Color::rgba(1.0, 1.0, 1.0, 0.0),
                    },
                ),
                GameOverText,
            ));
        });

    commands.insert_resource(GameOverAnim {
        elapsed: 0.0,
        music_spawned: false,
    });
}

fn stop_main_music(mut commands: Commands, main_music_q: Query<Entity, With<MusicMain>>) {
    for entity in main_music_q.iter() {
        if let Some(mut e) = commands.get_entity(entity) { e.despawn(); }
    }
}

const DELAY: f32 = 1.5;
const ANIM_DURATION: f32 = 6.0;

// --- Animation ---

fn animate_gameover(
    mut anim: ResMut<GameOverAnim>,
    time: Res<Time>,
    mut text_q: Query<(&mut Text, &mut Transform), With<GameOverText>>,
    mut bg_q: Query<&mut BackgroundColor, With<GameOverBackground>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    anim.elapsed += time.delta_seconds();

    // rien avant le délai de 2 secondes
    if anim.elapsed < DELAY {
        return;
    }

    // musique et animation démarrent ensemble à 2s
    if !anim.music_spawned {
        anim.music_spawned = true;
        commands.spawn((
            AudioBundle {
                source: asset_server.load("audio/sfx/you_died.ogg"),
                settings: PlaybackSettings::ONCE,
            },
            MusicGameOver,
        ));
    }

    // progression calculée depuis le début de l'animation (après le délai)
    let progress = ((anim.elapsed - DELAY) / ANIM_DURATION).clamp(0.0, 1.0);

    // fond : noir opaque → semi-transparent
    if let Ok(mut bg) = bg_q.get_single_mut() {
        bg.0.set_a(1.0 - progress * 0.25);
    }

    // texte : opacité 0 → 1, zoom 0.3 → 1.0
    for (mut text, mut transform) in text_q.iter_mut() {
        for section in text.sections.iter_mut() {
            section.style.color.set_a(progress);
        }
        let scale = 0.3 + progress * 0.7;
        transform.scale = Vec3::splat(scale);
    }
}

// --- Cleanup ---

fn cleanup_gameover_ui(mut commands: Commands, query: Query<Entity, With<GameOverUI>>) {
    for entity in query.iter() {
        if let Some(e) = commands.get_entity(entity) { e.despawn_recursive(); }
    }
}

fn cleanup_gameover_anim(mut commands: Commands) {
    commands.remove_resource::<GameOverAnim>();
}

// --- Restart ---

fn handle_restart(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut commands: Commands,
    gameover_music_q: Query<Entity, With<MusicGameOver>>,
    play_mode: Option<Res<PlayMode>>,
    confirm: Option<ResMut<ConfirmPopup>>,
    confirm_ui_q: Query<Entity, With<ConfirmPopupUI>>,
    mut confirm_text_q: Query<(&mut Text, &ConfirmOptionMarker)>,
    asset_server: Res<AssetServer>,
) {
    // ─── Popup de confirmation active ───────────────────────────
    if let Some(mut popup) = confirm {
        // Mise à jour des couleurs Oui/Non
        for (mut text, marker) in confirm_text_q.iter_mut() {
            let is_sel = marker.0 == popup.selected;
            for section in text.sections.iter_mut() {
                if is_sel {
                    section.style.color = Color::rgba(1.0, 0.85, 0.0, 1.0);
                } else {
                    section.style.color = Color::rgba(0.6, 0.6, 0.6, 1.0);
                }
            }
        }

        if keyboard.just_pressed(KeyCode::ArrowLeft) || keyboard.just_pressed(KeyCode::KeyQ) {
            popup.selected = 0;
        }
        if keyboard.just_pressed(KeyCode::ArrowRight) || keyboard.just_pressed(KeyCode::KeyD) {
            popup.selected = 1;
        }

        if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
            if popup.selected == 1 {
                // Oui → abandon campagne
                commands.remove_resource::<CampaignProgress>();
                commands.remove_resource::<PlayMode>();
                commands.remove_resource::<ConfirmPopup>();
                despawn_confirm_popup(&mut commands, &confirm_ui_q);
                next_state.set(GameState::MainMenu);
            } else {
                // Non → fermer la popup
                commands.remove_resource::<ConfirmPopup>();
                despawn_confirm_popup(&mut commands, &confirm_ui_q);
            }
        }

        if keyboard.just_pressed(KeyCode::Escape) {
            commands.remove_resource::<ConfirmPopup>();
            despawn_confirm_popup(&mut commands, &confirm_ui_q);
        }

        return;
    }

    // ─── R = rejouer le niveau ──────────────────────────────────
    if keyboard.just_pressed(KeyCode::KeyR) {
        for entity in gameover_music_q.iter() {
            if let Some(mut e) = commands.get_entity(entity) { e.despawn(); }
        }
        next_state.set(GameState::Playing);
    }

    // ─── Echap = quitter ────────────────────────────────────────
    if keyboard.just_pressed(KeyCode::Escape) {
        let is_campaign = play_mode.map(|m| *m) == Some(PlayMode::Campaign);
        if is_campaign {
            commands.insert_resource(ConfirmPopup { selected: 0 });
            spawn_confirm_popup(&mut commands, &asset_server);
        } else {
            commands.remove_resource::<PlayMode>();
            for entity in gameover_music_q.iter() {
                if let Some(mut e) = commands.get_entity(entity) { e.despawn(); }
            }
            next_state.set(GameState::MainMenu);
        }
    }
}
