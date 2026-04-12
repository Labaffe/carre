//! Écran de sélection de niveaux — style "Bounty Network".
//!
//! Le background affiche la salle de commandement. Par-dessus, une UI Bevy
//! construit l'écran interactif : titre, grille 2×2 de cartes de prime,
//! et footer d'instructions. La navigation est directionnelle (ZQSD / flèches).

use crate::game::{
    CampaignProgress, ConfirmOptionMarker, ConfirmPopup, ConfirmPopupUI, GameProgress, PlayMode,
    despawn_confirm_popup, spawn_confirm_popup,
};
use crate::level::level_name;
use crate::mainmenu::MainMenuMusic;
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

/// Carte de prime cliquable dans la grille.
#[derive(Component)]
struct PrimeCard {
    index: usize, // index 0-based (grille 2×2 : 0=haut-gauche, 1=haut-droite, 2=bas-gauche, 3=bas-droite)
}

/// Bordure lumineuse autour d'une carte (pour animation de sélection).
#[derive(Component)]
struct CardBorder(usize);

/// Label sous la carte.
#[derive(Component)]
struct CardLabel(usize);

/// Texte du footer.
#[derive(Component)]
struct FooterText;


// ─── Ressources ─────────────────────────────────────────────────────

#[derive(Resource)]
struct LevelSelectState {
    selected: usize,
    total: usize,       // nombre de cartes affichées
    elapsed: f32,        // pour animations
}

/// Textures normal/selected pour chaque carte.
#[derive(Resource)]
struct CardTextures {
    normal: Vec<Handle<Image>>,
    selected: Vec<Handle<Image>>,
}

// ─── Constantes ─────────────────────────────────────────────────────

/// Positions des 4 slots en screen-space (left, top, width, height).
const CARD_SLOTS: [(f32, f32, f32, f32); 4] = [
    (779.0, 308.0, 140.0, 132.0), // haut-gauche
    (942.0, 308.0, 140.0, 132.0), // haut-droite
    (779.0, 456.0, 140.0, 130.0), // bas-gauche
    (942.0, 454.0, 140.0, 132.0), // bas-droite
];

/// Couleur de la bordure sélectionnée (cyan néon).
const SELECTED_BORDER: Color = Color::rgba(0.0, 1.0, 1.0, 1.0);
/// Couleur de la bordure non sélectionnée.
const NORMAL_BORDER: Color = Color::rgba(0.3, 0.4, 0.5, 0.6);
/// Couleur de la bordure d'un niveau complété.
const COMPLETED_BORDER: Color = Color::rgba(0.2, 0.2, 0.2, 0.6);

// ─── Setup ──────────────────────────────────────────────────────────

fn setup_level_select(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    play_mode: Option<Res<PlayMode>>,
    progress: Res<GameProgress>,
    campaign: Option<Res<CampaignProgress>>,
    windows: Query<&Window>,
    existing_music: Query<Entity, With<MainMenuMusic>>,
) {
    let font = asset_server.load("fonts/PressStart2P-Regular.ttf");
    let mode = play_mode.map(|m| *m).unwrap_or(PlayMode::Primes);

    let footer = match mode {
        PlayMode::Campaign => "ENTREE : JOUER",
        PlayMode::Primes => "ENTREE : JOUER  |  ECHAP : RETOUR",
    };

    // Niveaux complétés (uniquement en Campagne)
    let completed: std::collections::HashSet<usize> = match mode {
        PlayMode::Campaign => campaign.map(|c| c.completed.clone()).unwrap_or_default(),
        PlayMode::Primes => std::collections::HashSet::new(),
    };

    // Toujours afficher 4 slots (les niveaux non-définis utilisent empty_prime)
    let total = 4;
    let first_available = (0..progress.total_levels.min(4))
        .find(|i| !completed.contains(&(i + 1)))
        .unwrap_or(0);

    // ── Background (sprite world-space) ─────────────────────────
    let window = windows.single();
    let img_w = 1536.0_f32;
    let img_h = 672.0_f32;
    let scale = (window.width() / img_w).max(window.height() / img_h);
    commands.spawn((
        SpriteBundle {
            texture: asset_server.load("images/backgrounds/prime_selection_background_2.png"),
            transform: Transform {
                translation: Vec3::new(0.0, 0.0, 5.0),
                scale: Vec3::splat(scale),
                ..default()
            },
            ..default()
        },
        LevelSelectUI,
    ));

    // ── Musique du menu (relancer si elle ne tourne pas) ──────────
    if existing_music.is_empty() {
        commands.spawn((
            AudioBundle {
                source: asset_server.load("audio/music/main_menu.ogg"),
                settings: PlaybackSettings::LOOP,
            },
            MainMenuMusic,
        ));
    }

    // ── Cartes positionnées en absolu sur les slots du background ──
    // Noms des sprites par slot (sans extension)
    let card_names = [
        "prime_space_invader",
        "empty_prime",
        "empty_prime",
        "empty_prime",
    ];
    let normal_textures: Vec<Handle<Image>> = card_names.iter()
        .map(|name| asset_server.load(format!("images/primes/{}.png", name)))
        .collect();
    let selected_textures: Vec<Handle<Image>> = card_names.iter()
        .map(|name| asset_server.load(format!("images/primes/{}_selected.png", name)))
        .collect();

    for i in 0..4 {
        let (left, top, w, h) = CARD_SLOTS[i];
        let level_num = i + 1;
        let name = level_name(level_num);

        // Carte
        commands.spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(left),
                    top: Val::Px(top),
                    width: Val::Px(w),
                    height: Val::Px(h),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: Color::NONE.into(),
                ..default()
            },
            LevelSelectUI,
            CardBorder(i),
        ))
        .with_children(|slot| {
            slot.spawn((
                ImageBundle {
                    image: UiImage::new(if i == first_available {
                        selected_textures[i].clone()
                    } else {
                        normal_textures[i].clone()
                    }),
                    style: Style {
                        width: Val::Px(w),
                        height: Val::Px(h),
                        ..default()
                    },
                    ..default()
                },
                LevelSelectUI,
                PrimeCard { index: i },
            ));
        });

        // Label sous la carte
        commands.spawn((
            TextBundle {
                text: Text::from_section(
                    name,
                    TextStyle {
                        font: font.clone(),
                        font_size: 10.0,
                        color: Color::rgba(0.7, 0.8, 0.8, 0.9),
                    },
                ),
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(left + 3.0),
                    top: Val::Px(top + h + 3.0),
                    width: Val::Px(w),
                    justify_self: JustifySelf::Center,
                    ..default()
                },
                ..default()
            },
            LevelSelectUI,
            CardLabel(i),
        ));
    }

    // ── Footer (en bas, centré) — masqué pour calibrage ─────────
    commands.spawn((
        TextBundle {
            text: Text::from_section(
                footer,
                TextStyle {
                    font: font.clone(),
                    font_size: 14.0,
                    color: Color::rgba(0.5, 0.6, 0.6, 0.8),
                },
            ),
            style: Style {
                position_type: PositionType::Absolute,
                bottom: Val::Px(40.0),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                justify_self: JustifySelf::Center,
                ..default()
            },
            ..default()
        },
        LevelSelectUI,
        FooterText,
    ));

    commands.insert_resource(CardTextures {
        normal: normal_textures,
        selected: selected_textures,
    });
    commands.insert_resource(LevelSelectState {
        selected: first_available,
        total,
        elapsed: 0.0,
    });
}


// ─── Animation ──────────────────────────────────────────────────────

fn animate_level_select(
    mut state: ResMut<LevelSelectState>,
    time: Res<Time>,
    play_mode: Option<Res<PlayMode>>,
    campaign: Option<Res<CampaignProgress>>,
    mut border_q: Query<(&CardBorder, &mut BackgroundColor), Without<PrimeCard>>,
    mut label_q: Query<(&CardLabel, &mut Text)>,
    mut card_q: Query<(&PrimeCard, &mut UiImage, &mut BackgroundColor), Without<CardBorder>>,
    textures: Res<CardTextures>,
) {
    state.elapsed += time.delta_seconds();
    let mode = play_mode.map(|m| *m).unwrap_or(PlayMode::Primes);

    let completed: std::collections::HashSet<usize> = match mode {
        PlayMode::Campaign => campaign.map(|c| c.completed.clone()).unwrap_or_default(),
        PlayMode::Primes => std::collections::HashSet::new(),
    };

    // Pulsation de la bordure sélectionnée
    let pulse = ((state.elapsed * 3.0).sin() * 0.3 + 0.7).clamp(0.4, 1.0);

    for (border, mut bg) in border_q.iter_mut() {
        let idx = border.0;
        let level_num = idx + 1;
        let is_completed = completed.contains(&level_num);
        let is_selected = idx == state.selected;
        let has_level = idx < state.total;

        if !has_level {
            bg.0 = Color::rgba(0.1, 0.1, 0.1, 0.3);
        } else if is_completed {
            bg.0 = COMPLETED_BORDER;
        } else if is_selected {
            bg.0 = Color::rgba(0.0, pulse, pulse, 1.0);
        } else {
            bg.0 = NORMAL_BORDER;
        }
    }

    // Texture normal/selected + filtre grisé si complété
    for (card, mut image, mut bg) in card_q.iter_mut() {
        let level_num = card.index + 1;
        let is_selected = card.index == state.selected;
        let is_completed = completed.contains(&level_num);

        if is_selected {
            image.texture = textures.selected[card.index].clone();
        } else {
            image.texture = textures.normal[card.index].clone();
        }

        if is_completed {
            bg.0 = Color::rgba(0.6, 0.6, 0.6, 1.0);
        } else {
            bg.0 = Color::WHITE;
        }
    }

    // Couleur du label selon l'état
    for (label, mut text) in label_q.iter_mut() {
        let level_num = label.0 + 1;
        let is_selected = label.0 == state.selected;
        let is_completed = completed.contains(&level_num);
        for section in text.sections.iter_mut() {
            section.style.color = if is_completed {
                Color::rgba(0.18, 0.541, 0.525, 1.0)
            } else if is_selected {
                Color::rgba(0.659, 1.0, 0.984, 1.0)
            } else {
                Color::rgba(0.31, 0.949, 0.933, 1.0)
            };
        }
    }
}

// ─── Input (navigation grille 2×2) ─────────────────────────────────

fn handle_level_select_input(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut state: Option<ResMut<LevelSelectState>>,
    play_mode: Option<Res<PlayMode>>,
    campaign: Option<Res<CampaignProgress>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut progress: ResMut<GameProgress>,
    menu_music_q: Query<Entity, With<MainMenuMusic>>,
    mut confirm: Option<ResMut<ConfirmPopup>>,
    confirm_ui_q: Query<Entity, With<ConfirmPopupUI>>,
    mut confirm_options_q: Query<(&ConfirmOptionMarker, &mut Text)>,
    asset_server: Res<AssetServer>,
) {
    let Some(ref mut state) = state else { return };
    let mode = play_mode.map(|m| *m).unwrap_or(PlayMode::Primes);
    let total = state.total;

    let completed: std::collections::HashSet<usize> = match mode {
        PlayMode::Campaign => campaign.map(|c| c.completed.clone()).unwrap_or_default(),
        PlayMode::Primes => std::collections::HashSet::new(),
    };

    // ── Popup de confirmation active ────────────────────────────
    if let Some(ref mut confirm) = confirm {
        let ui_yellow = Color::rgba(1.0, 0.85, 0.0, 1.0);

        // Navigation gauche/droite
        if keyboard.just_pressed(KeyCode::ArrowLeft) || keyboard.just_pressed(KeyCode::KeyQ) {
            confirm.selected = 0;
        }
        if keyboard.just_pressed(KeyCode::ArrowRight) || keyboard.just_pressed(KeyCode::KeyD) {
            confirm.selected = 1;
        }

        // Mise à jour des couleurs
        for (marker, mut text) in confirm_options_q.iter_mut() {
            for section in text.sections.iter_mut() {
                if marker.0 == confirm.selected {
                    section.style.color = ui_yellow;
                } else {
                    section.style.color = Color::rgba(0.6, 0.6, 0.6, 1.0);
                }
            }
        }

        if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
            if confirm.selected == 1 {
                // Oui → quitter
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

    // Helper : un slot est sélectionnable s'il n'est pas complété
    let is_available = |idx: usize| -> bool {
        idx < total && !completed.contains(&(idx + 1))
    };

    // Navigation grille 2×2
    //  0  1
    //  2  3
    if keyboard.just_pressed(KeyCode::ArrowLeft) || keyboard.just_pressed(KeyCode::KeyQ) {
        let target = if state.selected % 2 == 1 { state.selected - 1 } else { state.selected };
        if target != state.selected && is_available(target) {
            state.selected = target;
        }
    }
    if keyboard.just_pressed(KeyCode::ArrowRight) || keyboard.just_pressed(KeyCode::KeyD) {
        let target = if state.selected % 2 == 0 && state.selected + 1 < total { state.selected + 1 } else { state.selected };
        if target != state.selected && is_available(target) {
            state.selected = target;
        }
    }
    if keyboard.just_pressed(KeyCode::ArrowUp) || keyboard.just_pressed(KeyCode::KeyZ) {
        let target = if state.selected >= 2 { state.selected - 2 } else { state.selected };
        if target != state.selected && is_available(target) {
            state.selected = target;
        }
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) || keyboard.just_pressed(KeyCode::KeyS) {
        let target = if state.selected + 2 < total { state.selected + 2 } else { state.selected };
        if target != state.selected && is_available(target) {
            state.selected = target;
        }
    }

    // Lancer un niveau (bloqué si complété en Campagne)
    if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
        let level_num = state.selected + 1;
        if level_num <= total && !completed.contains(&level_num) {
            // Arrêter la musique du menu
            for entity in menu_music_q.iter() {
                if let Some(e) = commands.get_entity(entity) {
                    e.despawn_recursive();
                }
            }
            progress.current_level = level_num;
            next_state.set(GameState::Playing);
        }
    }

    // Escape — confirmation si campagne avec progression, sinon retour direct
    if keyboard.just_pressed(KeyCode::Escape) {
        let has_progress = !completed.is_empty();

        match mode {
            PlayMode::Primes => {
                commands.remove_resource::<PlayMode>();
                next_state.set(GameState::MainMenu);
            }
            PlayMode::Campaign if has_progress => {
                // Afficher la popup de confirmation
                commands.insert_resource(ConfirmPopup { selected: 0 });
                spawn_confirm_popup(&mut commands, &asset_server);
            }
            PlayMode::Campaign => {
                commands.remove_resource::<PlayMode>();
                next_state.set(GameState::MainMenu);
            }
        }
    }
}

// ─── Cleanup ────────────────────────────────────────────────────────

fn cleanup_level_select(
    mut commands: Commands,
    ui_q: Query<(Entity, Option<&Parent>), With<LevelSelectUI>>,
) {
    commands.remove_resource::<LevelSelectState>();
    commands.remove_resource::<CardTextures>();
    // Ne despawn que les entités racine — les enfants suivent via despawn_recursive
    for (entity, parent) in ui_q.iter() {
        if parent.is_none() {
            if let Some(e) = commands.get_entity(entity) {
                e.despawn_recursive();
            }
        }
    }
}
