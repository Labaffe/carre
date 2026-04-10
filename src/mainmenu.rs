//! Écran de menu principal.
//!
//! - Fond noir pendant 1 seconde, puis fondu d'apparition sur 1 seconde.
//! - Affiche le logo `main_menu_title.png` au centre sur fond noir.
//! - Trois options : "Commencer", "Paramètres" et "Quitter".
//! - Sous-menu Paramètres : réglage du volume global.

use crate::state::GameState;
use crate::GameSettings;
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
    Settings,
    Quit,
}

/// Marqueur pour les éléments du sous-menu Paramètres.
#[derive(Component)]
struct SettingsUI;

/// Texte affichant la valeur du volume.
#[derive(Component)]
struct VolumeText;

/// Vue active du menu.
#[derive(Clone, PartialEq)]
enum MenuView {
    Main,
    Settings,
}

#[derive(Resource)]
struct MainMenuAnim {
    elapsed: f32,
    selected: usize,
    view: MenuView,
}

// ─── Constantes ──────────────────────────────────────────────────────

const FADE_DELAY: f32 = 1.0;
const FADE_DURATION: f32 = 1.0;
const TILE_SIZE: f32 = 128.0;
/// Pas d'incrément du volume (5%).
const VOLUME_STEP: f32 = 0.05;

// ─── Setup ───────────────────────────────────────────────────────────

fn setup_main_menu(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform, &OrthographicProjection)>,
) {
    let font = asset_server.load("fonts/PressStart2P-Regular.ttf");
    let tile_texture = asset_server.load("images/space_tile_1.png");

    // ── Tiles de fond (world-space sprites) ───────────────────────
    let (half_w, half_h) = if let Ok((_cam, _gt, proj)) = camera_q.get_single() {
        (proj.area.max.x, proj.area.max.y)
    } else {
        let window = windows.single();
        (window.width() / 2.0, window.height() / 2.0)
    };

    let rotations = [0.0_f32, 90.0, 180.0, 270.0];
    let margin = TILE_SIZE;
    let total_w = (half_w + margin) * 2.0;
    let total_h = (half_h + margin) * 2.0;
    let cols = (total_w / TILE_SIZE).ceil() as i32;
    let rows = (total_h / TILE_SIZE).ceil() as i32;
    let start_x = -(half_w + margin);
    let start_y = -(half_h + margin);

    for row in 0..rows {
        for col in 0..cols {
            let x = start_x + col as f32 * TILE_SIZE + TILE_SIZE / 2.0;
            let y = start_y + row as f32 * TILE_SIZE + TILE_SIZE / 2.0;
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
            // Logo
            parent.spawn((
                ImageBundle {
                    image: UiImage::new(asset_server.load("images/main_menu_title.png")),
                    style: Style {
                        width: Val::Px(750.0),
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

            // Option : Paramètres
            parent.spawn((
                TextBundle::from_section(
                    "Paramètres",
                    TextStyle {
                        font: font.clone(),
                        font_size: 42.0,
                        color: Color::rgba(1.0, 1.0, 1.0, 0.0),
                    },
                ),
                MenuOption {
                    action: MenuAction::Settings,
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
        view: MenuView::Main,
    });
}

// ─── Animation ───────────────────────────────────────────────────────

fn animate_main_menu(
    mut anim: ResMut<MainMenuAnim>,
    time: Res<Time>,
    root_q: Query<&Children, With<MainMenuRoot>>,
    mut bg_root_q: Query<&mut BackgroundColor, With<MainMenuRoot>>,
    mut logo_q: Query<
        &mut BackgroundColor,
        (
            With<MainMenuUI>,
            Without<MainMenuRoot>,
            Without<MenuOption>,
            Without<Text>,
        ),
    >,
    mut text_q: Query<(&mut Text, &MenuOption, &mut Style)>,
    mut tile_q: Query<&mut Sprite, With<MainMenuTile>>,
    mut volume_text_q: Query<&mut Text, (With<VolumeText>, Without<MenuOption>)>,
    settings: Res<GameSettings>,
) {
    anim.elapsed += time.delta_seconds();

    let alpha = if anim.elapsed < FADE_DELAY {
        0.0
    } else {
        ((anim.elapsed - FADE_DELAY) / FADE_DURATION).clamp(0.0, 1.0)
    };

    // Tiles
    for mut sprite in tile_q.iter_mut() {
        sprite.color.set_a(alpha);
    }

    // Fond noir du root
    for mut bg in bg_root_q.iter_mut() {
        bg.0.set_a(1.0 - alpha);
    }

    // Logo
    for mut bg in logo_q.iter_mut() {
        bg.0.set_a(alpha);
    }

    // Menu options — visibilité dépend de la vue active
    let mut idx = 0;
    for (mut text, _option, mut style) in text_q.iter_mut() {
        if anim.view == MenuView::Settings {
            // Cacher les options du menu principal quand on est dans Paramètres
            style.display = Display::None;
        } else {
            style.display = Display::Flex;
            let is_selected = idx == anim.selected;
            for section in text.sections.iter_mut() {
                if is_selected {
                    section.style.color = Color::rgba(1.0, 0.85, 0.0, alpha);
                } else {
                    section.style.color = Color::rgba(0.6, 0.6, 0.6, alpha);
                }
            }
        }
        idx += 1;
    }

    // Mettre à jour le texte du volume dans le sous-menu
    for mut text in volume_text_q.iter_mut() {
        let pct = (settings.master_volume * 100.0).round() as i32;
        text.sections[0].value = format!("< Volume : {} % >", pct);
    }
}

// ─── Input ───────────────────────────────────────────────────────────

fn handle_menu_input(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut anim: ResMut<MainMenuAnim>,
    mut next_state: ResMut<NextState<GameState>>,
    mut exit: EventWriter<AppExit>,
    mut settings: ResMut<GameSettings>,
    mut global_volume: ResMut<GlobalVolume>,
    asset_server: Res<AssetServer>,
    settings_ui_q: Query<Entity, With<SettingsUI>>,
    root_q: Query<Entity, With<MainMenuRoot>>,
) {
    if anim.elapsed < FADE_DELAY {
        return;
    }

    match anim.view {
        MenuView::Main => {
            handle_main_view(
                &keyboard,
                &mouse,
                &mut anim,
                &mut next_state,
                &mut exit,
                &mut commands,
                &asset_server,
                &settings,
                &root_q,
            );
        }
        MenuView::Settings => {
            handle_settings_view(
                &keyboard,
                &mut anim,
                &mut settings,
                &mut global_volume,
                &mut commands,
                &settings_ui_q,
            );
        }
    }
}

fn handle_main_view(
    keyboard: &Res<ButtonInput<KeyCode>>,
    mouse: &Res<ButtonInput<MouseButton>>,
    anim: &mut ResMut<MainMenuAnim>,
    next_state: &mut ResMut<NextState<GameState>>,
    exit: &mut EventWriter<AppExit>,
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    settings: &ResMut<GameSettings>,
    root_q: &Query<Entity, With<MainMenuRoot>>,
) {
    // Navigation
    if keyboard.just_pressed(KeyCode::ArrowUp) || keyboard.just_pressed(KeyCode::KeyW) {
        if anim.selected > 0 {
            anim.selected -= 1;
        }
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) || keyboard.just_pressed(KeyCode::KeyS) {
        if anim.selected < 2 {
            anim.selected += 1;
        }
    }

    // Validation
    if keyboard.just_pressed(KeyCode::Enter)
        || keyboard.just_pressed(KeyCode::Space)
        || mouse.just_pressed(MouseButton::Left)
    {
        match anim.selected {
            0 => {
                next_state.set(GameState::Playing);
            }
            1 => {
                // Ouvrir le sous-menu Paramètres
                anim.view = MenuView::Settings;
                anim.selected = 0;
                spawn_settings_ui(commands, asset_server, settings, root_q);
            }
            2 => {
                exit.send(AppExit);
            }
            _ => {}
        }
    }
}

fn handle_settings_view(
    keyboard: &Res<ButtonInput<KeyCode>>,
    anim: &mut ResMut<MainMenuAnim>,
    settings: &mut ResMut<GameSettings>,
    global_volume: &mut ResMut<GlobalVolume>,
    commands: &mut Commands,
    settings_ui_q: &Query<Entity, With<SettingsUI>>,
) {
    // Gauche/Droite pour ajuster le volume
    if keyboard.just_pressed(KeyCode::ArrowLeft) || keyboard.just_pressed(KeyCode::KeyA) {
        settings.master_volume = (settings.master_volume - VOLUME_STEP).max(0.0);
        global_volume.volume = bevy::audio::Volume::new(settings.master_volume);
    }
    if keyboard.just_pressed(KeyCode::ArrowRight) || keyboard.just_pressed(KeyCode::KeyD) {
        settings.master_volume = (settings.master_volume + VOLUME_STEP).min(1.0);
        global_volume.volume = bevy::audio::Volume::new(settings.master_volume);
    }

    // Retour au menu principal
    if keyboard.just_pressed(KeyCode::Escape)
        || keyboard.just_pressed(KeyCode::Enter)
        || keyboard.just_pressed(KeyCode::Space)
    {
        anim.view = MenuView::Main;
        anim.selected = 1; // Reselect "Paramètres"
        // Despawn le sous-menu
        for entity in settings_ui_q.iter() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

/// Spawn l'UI du sous-menu Paramètres (enfant du root).
fn spawn_settings_ui(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    settings: &ResMut<GameSettings>,
    root_q: &Query<Entity, With<MainMenuRoot>>,
) {
    let font = asset_server.load("fonts/PressStart2P-Regular.ttf");
    let pct = (settings.master_volume * 100.0).round() as i32;

    let Ok(root_entity) = root_q.get_single() else {
        return;
    };

    commands.entity(root_entity).with_children(|parent| {
        // Conteneur du sous-menu
        parent
            .spawn((
                NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(40.0),
                        ..default()
                    },
                    ..default()
                },
                SettingsUI,
            ))
            .with_children(|parent| {
                // Titre
                parent.spawn(TextBundle::from_section(
                    "PARAMÈTRES",
                    TextStyle {
                        font: font.clone(),
                        font_size: 48.0,
                        color: Color::WHITE,
                    },
                ));

                // Volume
                parent.spawn((
                    TextBundle::from_section(
                        format!("< Volume : {} % >", pct),
                        TextStyle {
                            font: font.clone(),
                            font_size: 32.0,
                            color: Color::rgba(1.0, 0.85, 0.0, 1.0),
                        },
                    ),
                    VolumeText,
                ));

                // Instruction
                parent.spawn(TextBundle::from_section(
                    "Entrée pour revenir",
                    TextStyle {
                        font: font.clone(),
                        font_size: 20.0,
                        color: Color::rgba(0.5, 0.5, 0.5, 1.0),
                    },
                ));
            });
    });
}

// ─── Cleanup ─────────────────────────────────────────────────────────

fn cleanup_main_menu(mut commands: Commands, query: Query<Entity, With<MainMenuUI>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
    commands.remove_resource::<MainMenuAnim>();
}
