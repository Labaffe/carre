//! Écran de menu principal.
//!
//! - Fond noir pendant 1 seconde, puis fondu d'apparition sur 1 seconde.
//! - Affiche le logo `main_menu_title.png` au centre sur fond noir.
//! - Deux options : "Commencer" et "Quitter". Police : Space Goatesque.
//! - Musique `main_menu.ogg` en boucle dès le lancement.

use crate::state::GameState;
use bevy::app::AppExit;
use bevy::prelude::*;

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::MainMenu), setup_main_menu)
            .add_systems(OnExit(GameState::MainMenu), cleanup_main_menu)
            .add_systems(
                Update,
                (animate_main_menu, handle_menu_input).run_if(in_state(GameState::MainMenu)),
            );
    }
}

// ─── Composants ──────────────────────────────────────────────────────

#[derive(Component)]
struct MainMenuUI;

#[derive(Component)]
struct MainMenuMusic;

#[derive(Component)]
struct MenuOption {
    action: MenuAction,
}

#[derive(Clone, PartialEq)]
enum MenuAction {
    Play,
    Quit,
}

#[derive(Resource)]
struct MainMenuAnim {
    elapsed: f32,
    /// Index de l'option sélectionnée (0 = Commencer, 1 = Quitter).
    selected: usize,
}

// ─── Constantes ──────────────────────────────────────────────────────

/// Délai avant le début du fondu (secondes).
const FADE_DELAY: f32 = 1.0;
/// Durée du fondu d'apparition (secondes).
const FADE_DURATION: f32 = 1.0;

// ─── Setup ───────────────────────────────────────────────────────────

fn setup_main_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/PressStart2P-Regular.ttf");

    // Musique du menu
    commands.spawn((
        AudioBundle {
            source: asset_server.load("audio/main_menu.ogg"),
            settings: PlaybackSettings::LOOP,
        },
        MainMenuMusic,
        MainMenuUI,
    ));

    // UI racine (fond noir, recouvre tout l'écran)
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
            MainMenuUI,
        ))
        .with_children(|parent| {
            // Logo (invisible au départ, alpha = 0)
            parent.spawn((
                ImageBundle {
                    image: UiImage::new(asset_server.load("images/main_menu_title.png")),
                    style: Style {
                        width: Val::Px(650.0),
                        height: Val::Auto,
                        margin: UiRect::bottom(Val::Px(60.0)),
                        ..default()
                    },
                    background_color: Color::rgba(1.0, 1.0, 1.0, 0.0).into(),
                    ..default()
                },
                MainMenuUI,
            ));

            // Option : Commencer
            parent.spawn((
                TextBundle::from_section(
                    "Commencer",
                    TextStyle {
                        font: font.clone(),
                        font_size: 42.0,
                        color: Color::rgba(1.0, 1.0, 1.0, 0.0),
                    },
                ),
                MenuOption {
                    action: MenuAction::Play,
                },
                MainMenuUI,
            ));

            // Option : Quitter
            parent.spawn((
                TextBundle::from_section(
                    "Quitter",
                    TextStyle {
                        font: font.clone(),
                        font_size: 42.0,
                        color: Color::rgba(1.0, 1.0, 1.0, 0.0),
                    },
                ),
                MenuOption {
                    action: MenuAction::Quit,
                },
                MainMenuUI,
            ));
        });

    commands.insert_resource(MainMenuAnim {
        elapsed: 0.0,
        selected: 0,
    });
}

// ─── Animation ───────────────────────────────────────────────────────

fn animate_main_menu(
    mut anim: ResMut<MainMenuAnim>,
    time: Res<Time>,
    mut image_q: Query<
        &mut BackgroundColor,
        (With<MainMenuUI>, Without<MenuOption>, Without<Text>),
    >,
    mut text_q: Query<(&mut Text, &MenuOption)>,
) {
    anim.elapsed += time.delta_seconds();

    // Calcul de l'opacité : 0 avant FADE_DELAY, puis fondu linéaire sur FADE_DURATION
    let alpha = if anim.elapsed < FADE_DELAY {
        0.0
    } else {
        ((anim.elapsed - FADE_DELAY) / FADE_DURATION).clamp(0.0, 1.0)
    };

    // Appliquer l'alpha au logo et aux images
    for mut bg in image_q.iter_mut() {
        bg.0.set_a(alpha);
    }

    // Appliquer l'alpha au texte avec surbrillance de l'option sélectionnée
    for (mut text, option) in text_q.iter_mut() {
        let is_selected = (option.action == MenuAction::Play && anim.selected == 0)
            || (option.action == MenuAction::Quit && anim.selected == 1);

        for section in text.sections.iter_mut() {
            if is_selected {
                // Option sélectionnée : jaune
                section.style.color = Color::rgba(1.0, 0.85, 0.0, alpha);
            } else {
                // Option non sélectionnée : blanc
                section.style.color = Color::rgba(0.6, 0.6, 0.6, alpha);
            }
        }
    }
}

// ─── Input ───────────────────────────────────────────────────────────

fn handle_menu_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut anim: ResMut<MainMenuAnim>,
    mut next_state: ResMut<NextState<GameState>>,
    mut exit: EventWriter<AppExit>,
) {
    // Ne pas accepter d'input avant que le menu soit visible
    if anim.elapsed < FADE_DELAY {
        return;
    }

    // Navigation haut/bas
    if keyboard.just_pressed(KeyCode::ArrowUp) || keyboard.just_pressed(KeyCode::KeyW) {
        if anim.selected > 0 {
            anim.selected -= 1;
        }
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) || keyboard.just_pressed(KeyCode::KeyS) {
        if anim.selected < 1 {
            anim.selected += 1;
        }
    }

    // Validation
    if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
        match anim.selected {
            0 => {
                next_state.set(GameState::Playing);
            }
            1 => {
                exit.send(AppExit);
            }
            _ => {}
        }
    }
}

// ─── Cleanup ─────────────────────────────────────────────────────────

fn cleanup_main_menu(mut commands: Commands, query: Query<Entity, With<MainMenuUI>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
    commands.remove_resource::<MainMenuAnim>();
}
