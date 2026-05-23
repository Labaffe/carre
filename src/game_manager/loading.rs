//! Écran de chargement court entre la sélection de niveau et le gameplay.
//!
//! Routé par `levelselect`/`gameover restart` avant `GameState::Playing` :
//! ```ignore
//! next_state.set(GameState::Loading); // au lieu de Playing
//! ```
//!
//! L'écran tient `LOADING_DURATION` secondes (fond noir + texte
//! "CHARGEMENT..."). À l'expiration, transition automatique vers Playing.
//!
//! Pendant cette fenêtre, l'`AssetServer` continue les décodages en
//! arrière-plan (musique, images préchargées au Startup mais pas finies).
//! Le hitch de `OnEnter(Playing)` (setup massif + premier rendu) reste,
//! mais le joueur le voit après un écran "CHARGEMENT" plutôt qu'en pleine
//! navigation menu → c'est plus acceptable visuellement.

use bevy::prelude::*;

use crate::game_manager::state::GameState;

/// Durée minimale de l'écran de chargement (s). Assez court pour ne pas
/// frustrer, assez long pour donner du temps à l'asset_server.
const LOADING_DURATION: f32 = 0.5;

pub struct LoadingPlugin;

impl Plugin for LoadingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Loading), setup_loading_ui)
            .add_systems(OnExit(GameState::Loading), cleanup_loading_ui)
            .add_systems(Update, loading_tick.run_if(in_state(GameState::Loading)));
    }
}

/// Marker sur les entités de l'écran de chargement (despawn à OnExit).
#[derive(Component)]
struct LoadingUI;

/// Timer de durée du chargement. Inséré à OnEnter, retiré à OnExit.
#[derive(Resource)]
struct LoadingTimer(Timer);

fn setup_loading_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/PressStart2P-Regular.ttf");

    // Conteneur plein écran noir opaque qui masque tout ce qui était dessous.
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 1.0)),
            GlobalZIndex(1000),
            LoadingUI,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("CHARGEMENT..."),
                TextFont {
                    font,
                    font_size: 32.0,
                    ..default()
                },
                TextColor(Color::srgba(1.0, 0.85, 0.0, 1.0)),
            ));
        });

    commands.insert_resource(LoadingTimer(Timer::from_seconds(
        LOADING_DURATION,
        TimerMode::Once,
    )));
}

fn loading_tick(
    time: Res<Time>,
    mut timer: ResMut<LoadingTimer>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    timer.0.tick(time.delta());
    if timer.0.is_finished() {
        next_state.set(GameState::Playing);
    }
}

fn cleanup_loading_ui(mut commands: Commands, q: Query<Entity, With<LoadingUI>>) {
    for entity in q.iter() {
        if let Ok(mut e) = commands.get_entity(entity) {
            e.try_despawn();
        }
    }
    commands.remove_resource::<LoadingTimer>();
}
