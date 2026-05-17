//! Écran de menu principal.
//!
//! - Fond noir pendant 1 seconde, puis fondu d'apparition sur 1 seconde.
//! - Affiche le logo `main_menu_title.png` au centre sur fond noir.
//! - Trois options : "Commencer", "Paramètres" et "Quitter".
//! - Sous-menu Paramètres : réglage du volume global.

use crate::GameSettings;
use crate::game_manager::game::{CampaignProgress, PlayMode};
use crate::game_manager::state::GameState;
use bevy::app::AppExit;
use bevy::color::Alpha;
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
pub struct MainMenuMusic;

/// Tile de fond (world-space sprite).
#[derive(Component)]
struct MainMenuTile;

/// NodeBundle racine (fond noir, animé en inverse : 1→0).
#[derive(Component)]
struct MainMenuRoot;

/// Marqueur pour le logo du titre.
#[derive(Component)]
struct MainMenuLogo;

/// Conteneur des options du menu principal.
#[derive(Component)]
struct MenuOptionsContainer;

#[derive(Component)]
struct MenuOption {
    index: usize,
    action: MenuAction,
}

#[derive(Clone, PartialEq)]
enum MenuAction {
    Play,
    Primes,
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
/// Probabilité qu'une tile de fond soit invisible (0.0 = toutes visibles, 1.0 = toutes invisibles).
const TILE_HIDDEN_CHANCE: f64 = 0.2;

// ─── Setup ───────────────────────────────────────────────────────────

fn setup_main_menu(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform, &Projection)>,
    existing_music: Query<Entity, With<MainMenuMusic>>,
) {
    let font = asset_server.load("fonts/PressStart2P-Regular.ttf");
    let tile_texture = asset_server.load("images/backgrounds/space_tile_1.png");

    // ── Tiles de fond (world-space sprites) ───────────────────────
    let (half_w, half_h) = if let Ok((_cam, _gt, Projection::Orthographic(proj))) = camera_q.single() {
        (proj.area.max.x, proj.area.max.y)
    } else {
        let window = windows.single().unwrap();
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

            // Certaines tiles sont aléatoirement invisibles
            if fastrand::f64() < TILE_HIDDEN_CHANCE {
                continue;
            }

            commands.spawn((
                Sprite {
                    image: tile_texture.clone(),
                    custom_size: Some(Vec2::splat(TILE_SIZE)),
                    color: Color::srgba(1.0, 1.0, 1.0, 0.0),
                    ..default()
                },
                Transform {
                    translation: Vec3::new(x, y, 0.0),
                    rotation: Quat::from_rotation_z(angle_rad),
                    ..default()
                },
                MainMenuTile,
                MainMenuUI,
            ));
        }
    }

    // Musique du menu (ne pas re-spawner si elle tourne déjà)
    if existing_music.is_empty() {
        commands.spawn((
            (AudioPlayer::new(asset_server.load("audio/music/main_menu.ogg")), PlaybackSettings::LOOP),
            MainMenuMusic,
        ));
    }

    // UI racine (fond noir, recouvre tout l'écran)
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 1.0)),
            MainMenuUI,
            MainMenuRoot,
        ))
        .with_children(|parent| {
            // Logo (centré indépendamment)
            parent.spawn((
                ImageNode {
                    image: asset_server.load("images/ui/main_menu_title.png"),
                    color: Color::srgba(1.0, 1.0, 1.0, 0.0),
                    ..default()
                },
                Node {
                    width: Val::Px(750.0),
                    height: Val::Auto,
                    margin: UiRect::bottom(Val::Px(200.0)),
                    ..default()
                },
                MainMenuUI,
                MainMenuLogo,
            ));

            // Conteneur des options du menu (décalé vers le haut)
            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        bottom: Val::Px(300.0),
                        row_gap: Val::Px(10.0),
                        ..default()
                    },
                    MainMenuUI,
                    MenuOptionsContainer,
                ))
                .with_children(|menu| {
                    // Option : Commencer
                    menu.spawn((
                        (Text::new("Commencer"), TextFont { font: font.clone(), font_size: 36.0, ..default() }, TextColor(Color::srgba(1.0, 1.0, 1.0, 0.0))),
                        MenuOption { index: 0, action: MenuAction::Play },
                        MainMenuUI,
                    ));

                    // Option : Primes
                    menu.spawn((
                        (Text::new("Primes"), TextFont { font: font.clone(), font_size: 36.0, ..default() }, TextColor(Color::srgba(1.0, 1.0, 1.0, 0.0))),
                        MenuOption { index: 1, action: MenuAction::Primes },
                        MainMenuUI,
                    ));

                    // Option : Paramètres
                    menu.spawn((
                        (Text::new("Paramètres"), TextFont { font: font.clone(), font_size: 36.0, ..default() }, TextColor(Color::srgba(1.0, 1.0, 1.0, 0.0))),
                        MenuOption { index: 2, action: MenuAction::Settings },
                        MainMenuUI,
                    ));
                    menu.spawn((
                        (Text::new("Editeur"), TextFont { font: font.clone(), font_size: 36.0, ..default() }, TextColor(Color::srgba(1.0, 1.0, 1.0, 0.0))),
                        MenuOption { index: 3, action: MenuAction::Settings },
                        MainMenuUI,
                    ));
                    // Option : Quitter
                    menu.spawn((
                        (Text::new("Quitter"), TextFont { font: font.clone(), font_size: 36.0, ..default() }, TextColor(Color::srgba(1.0, 1.0, 1.0, 0.0))),
                        MenuOption { index: 4, action: MenuAction::Quit },
                        MainMenuUI,
                    ));
                });
        });

    // Indication F1 en haut à droite
    commands.spawn((
        Text::new("F1 : Debug Mode"),
        TextFont { font, font_size: 14.0, ..default() },
        TextColor(Color::srgba(0.4, 0.4, 0.4, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(15.0),
            right: Val::Px(15.0),
            ..default()
        },
        MainMenuUI,
    ));

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
    mut bg_root_q: Query<&mut BackgroundColor, With<MainMenuRoot>>,
    mut logo_q: Query<
        (&mut ImageNode, &mut Node),
        (With<MainMenuLogo>, Without<MainMenuRoot>),
    >,
    mut container_q: Query<&mut Node, (With<MenuOptionsContainer>, Without<MainMenuLogo>)>,
    mut text_q: Query<
        (&mut TextColor, &MenuOption),
        (Without<MainMenuLogo>, Without<MenuOptionsContainer>),
    >,
    mut tile_q: Query<&mut Sprite, With<MainMenuTile>>,
    mut volume_text_q: Query<&mut Text, (With<VolumeText>, Without<MenuOption>)>,
    settings: Res<GameSettings>,
) {
    anim.elapsed += time.delta_secs();

    let alpha = if anim.elapsed < FADE_DELAY {
        0.0
    } else {
        ((anim.elapsed - FADE_DELAY) / FADE_DURATION).clamp(0.0, 1.0)
    };

    // Tiles
    for mut sprite in tile_q.iter_mut() {
        sprite.color.set_alpha(alpha);
    }

    // Fond noir du root
    for mut bg in bg_root_q.iter_mut() {
        bg.0.set_alpha(1.0 - alpha);
    }

    // Logo — cacher dans les sous-menus, fade via tint de l'ImageNode
    for (mut image_node, mut style) in logo_q.iter_mut() {
        if anim.view != MenuView::Main {
            style.display = Display::None;
        } else {
            style.display = Display::Flex;
            image_node.color.set_alpha(alpha);
        }
    }

    // Conteneur des options — cacher dans les sous-menus
    for mut style in container_q.iter_mut() {
        if anim.view != MenuView::Main {
            style.display = Display::None;
        } else {
            style.display = Display::Flex;
        }
    }

    // Menu options — couleurs de sélection (utilise option.index, pas l'ordre de la query)
    for (mut text_color, option) in text_q.iter_mut() {
        if anim.view == MenuView::Main {
            let is_selected = option.index == anim.selected;
            text_color.0 = if is_selected {
                Color::srgba(1.0, 0.85, 0.0, alpha)
            } else {
                Color::srgba(0.6, 0.6, 0.6, alpha)
            };
        }
    }

    // Mettre à jour le texte du volume dans le sous-menu
    for mut text in volume_text_q.iter_mut() {
        let pct = (settings.master_volume * 100.0).round() as i32;
        **text = format!("< Volume : {} % >", pct);
    }
}

// ─── Input ───────────────────────────────────────────────────────────

fn handle_menu_input(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut anim: ResMut<MainMenuAnim>,
    mut next_state: ResMut<NextState<GameState>>,
    mut exit: MessageWriter<AppExit>,
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
    exit: &mut MessageWriter<AppExit>,
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    settings: &ResMut<GameSettings>,
    root_q: &Query<Entity, With<MainMenuRoot>>,
) {
    // Navigation (4 options : Commencer, Primes, Paramètres, Quitter)
    if keyboard.just_pressed(KeyCode::ArrowUp) || keyboard.just_pressed(KeyCode::KeyW) {
        if anim.selected > 0 {
            anim.selected -= 1;
        }
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) || keyboard.just_pressed(KeyCode::KeyS) {
        if anim.selected < 4 {
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
                // Commencer : mode Campagne → sélection de niveaux
                commands.insert_resource(PlayMode::Campaign);
                commands.insert_resource(CampaignProgress::default());
                next_state.set(GameState::LevelSelect);
            }
            1 => {
                // Primes : mode à la carte → sélection de niveaux
                commands.insert_resource(PlayMode::Primes);
                next_state.set(GameState::LevelSelect);
            }
            2 => {
                // Ouvrir le sous-menu Paramètres
                anim.view = MenuView::Settings;
                anim.selected = 0;
                spawn_settings_ui(commands, asset_server, settings, root_q);
            }
            3 => {
                // Ouvrir l'éditeur'
                next_state.set(GameState::Editor);
            }
            4 => {
                exit.write(AppExit::Success);
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
        global_volume.volume = bevy::audio::Volume::Linear(settings.master_volume);
    }
    if keyboard.just_pressed(KeyCode::ArrowRight) || keyboard.just_pressed(KeyCode::KeyD) {
        settings.master_volume = (settings.master_volume + VOLUME_STEP).min(1.0);
        global_volume.volume = bevy::audio::Volume::Linear(settings.master_volume);
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
            if let Ok(mut e) = commands.get_entity(entity) {
                e.despawn();
            }
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

    let Ok(root_entity) = root_q.single() else {
        return;
    };

    commands.entity(root_entity).with_children(|parent| {
        // Conteneur du sous-menu
        parent
            .spawn((
                (
            Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(40.0),
                        ..default()
                    },
        ),
                SettingsUI,
            ))
            .with_children(|parent| {
                // Titre
                parent.spawn((Text::new("PARAMÈTRES"), TextFont { font: font.clone(), font_size: 48.0, ..default() }, TextColor(Color::WHITE)));

                // Volume
                parent.spawn((
                    (Text::new(format!("< Volume : {} % >", pct)), TextFont { font: font.clone(), font_size: 32.0, ..default() }, TextColor(Color::srgba(1.0, 0.85, 0.0, 1.0))),
                    VolumeText,
                ));

                // Instruction
                parent.spawn((Text::new("Entrée pour revenir"), TextFont { font: font.clone(), font_size: 20.0, ..default() }, TextColor(Color::srgba(0.5, 0.5, 0.5, 1.0))));
            });
    });
}

// ─── Cleanup ─────────────────────────────────────────────────────────

fn cleanup_main_menu(mut commands: Commands, query: Query<Entity, With<MainMenuUI>>) {
    for entity in query.iter() {
        if let Ok(mut e) = commands.get_entity(entity) {
            e.despawn();
        }
    }
    commands.remove_resource::<MainMenuAnim>();
}
