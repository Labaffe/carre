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

/// Tile de fond (world-space sprite).
#[derive(Component)]
struct MainMenuTile;

/// NodeBundle racine (fond noir, animé en inverse : 1→0).
#[derive(Component)]
struct MainMenuRoot;

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

/// Taille d'une tile en pixels.
const TILE_SIZE: f32 = 128.0;

fn setup_main_menu(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    windows: Query<&Window>,
) {
    let font = asset_server.load("fonts/PressStart2P-Regular.ttf");
    let tile_texture = asset_server.load("images/space_tile_1.png");

    // ── Tiles de fond (world-space sprites) ───────────────────────
    let window = windows.single();
    let half_w = window.width() / 2.0;
    let half_h = window.height() / 2.0;
    let rotations = [0.0_f32, 90.0, 180.0, 270.0];

    let cols = (window.width() / TILE_SIZE).ceil() as i32 + 2;
    let rows = (window.height() / TILE_SIZE).ceil() as i32 + 2;

    for row in 0..rows {
        for col in 0..cols {
            let x = -half_w + col as f32 * TILE_SIZE + TILE_SIZE / 2.0;
            let y = -half_h + row as f32 * TILE_SIZE + TILE_SIZE / 2.0;
            let angle_rad = rotations[fastrand::usize(0..4)].to_radians();

            commands.spawn((
                SpriteBundle {
                    texture: tile_texture.clone(),
                    sprite: Sprite {
                        custom_size: Some(Vec2::splat(TILE_SIZE)),
                        color: Color::rgba(1.0, 1.0, 1.0, 0.0),
                        ..default()
                    },
                    transform: Transform {
                        translation: Vec3::new(x, y, 0.0),
                        rotation: Quat::from_rotation_z(angle_rad),
                        ..default()
                    },
                    ..default()
                },
                MainMenuTile,
                MainMenuUI,
            ));
        }
    }

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
            MainMenuRoot,
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
    // Fond noir du root (fade-out 1→0)
    mut root_q: Query<&mut BackgroundColor, With<MainMenuRoot>>,
    // Logo (fade-in 0→1, exclut le root)
    mut logo_q: Query<
        &mut BackgroundColor,
        (With<MainMenuUI>, Without<MainMenuRoot>, Without<MenuOption>, Without<Text>),
    >,
    mut text_q: Query<(&mut Text, &MenuOption)>,
    mut tile_q: Query<&mut Sprite, With<MainMenuTile>>,
) {
    anim.elapsed += time.delta_seconds();

    // Calcul de l'opacité : 0 avant FADE_DELAY, puis fondu linéaire sur FADE_DURATION
    let alpha = if anim.elapsed < FADE_DELAY {
        0.0
    } else {
        ((anim.elapsed - FADE_DELAY) / FADE_DURATION).clamp(0.0, 1.0)
    };

    // Tiles : alpha 0→1
    for mut sprite in tile_q.iter_mut() {
        sprite.color.set_a(alpha);
    }

    // Fond noir du root : 1→0 (se dissipe, révèle les tiles)
    for mut bg in root_q.iter_mut() {
        bg.0.set_a(1.0 - alpha);
    }

    // Logo : 0→1 (apparaît)
    for mut bg in logo_q.iter_mut() {
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
    mouse: Res<ButtonInput<MouseButton>>,
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

    // Validation (clavier ou clic souris)
    if keyboard.just_pressed(KeyCode::Enter)
        || keyboard.just_pressed(KeyCode::Space)
        || mouse.just_pressed(MouseButton::Left)
    {
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
