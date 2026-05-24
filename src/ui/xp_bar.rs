//! Jauge d'expérience — barre horizontale en haut-centre de l'écran.
//!
//! Affiche `LV X [====     ] cur / next` où la barre se remplit selon
//! `Experience::progress_in_level()`. Quand la barre est pleine, le système
//! `watch_experience` (dans `game_manager::level_up`) déclenche le modal de
//! choix de carte ; après pioche, `level` est incrémenté et la barre repart
//! du début pour le palier suivant.

use bevy::prelude::*;

use crate::game_manager::level_up::Experience;
use crate::game_manager::state::GameState;

pub struct XpBarPlugin;

impl Plugin for XpBarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), setup_xp_bar)
            .add_systems(Update, update_xp_bar.run_if(in_state(GameState::Playing)));
    }
}

#[derive(Component)]
#[require(crate::GameplayEntity)]
struct XpBarRoot;

#[derive(Component)]
struct XpLevelText;

#[derive(Component)]
struct XpBarFill;

#[derive(Component)]
struct XpProgressText;

const FILL_COLOR: Color = Color::srgba(0.4, 0.75, 1.0, 1.0);
const BORDER_COLOR: Color = Color::srgba(0.5, 0.7, 1.0, 1.0);
const TRACK_COLOR: Color = Color::srgba(0.05, 0.05, 0.12, 0.85);
const LABEL_COLOR: Color = Color::srgba(0.75, 0.9, 1.0, 1.0);

fn setup_xp_bar(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/PressStart2P-Regular.ttf");

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(75.0),
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                ..default()
            },
            XpBarRoot,
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("LV 0"),
                TextFont { font: font.clone(), font_size: 18.0, ..default() },
                TextColor(LABEL_COLOR),
                XpLevelText,
            ));
            p.spawn((
                Node {
                    width: Val::Px(380.0),
                    height: Val::Px(14.0),
                    border: UiRect::all(Val::Px(2.0)),
                    padding: UiRect::all(Val::Px(0.0)),
                    ..default()
                },
                BorderColor::all(BORDER_COLOR),
                BackgroundColor(TRACK_COLOR),
            ))
            .with_children(|bar| {
                bar.spawn((
                    Node {
                        width: Val::Percent(0.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(FILL_COLOR),
                    XpBarFill,
                ));
            });
            p.spawn((
                Text::new("0 / 50"),
                TextFont { font: font.clone(), font_size: 14.0, ..default() },
                TextColor(Color::srgba(0.75, 0.9, 1.0, 0.8)),
                XpProgressText,
            ));
        });
}

fn update_xp_bar(
    experience: Res<Experience>,
    mut level_q: Query<&mut Text, (With<XpLevelText>, Without<XpProgressText>)>,
    mut progress_q: Query<&mut Text, (With<XpProgressText>, Without<XpLevelText>)>,
    mut fill_q: Query<&mut Node, With<XpBarFill>>,
) {
    let lo = experience.current_threshold();
    let hi = experience.next_threshold();
    let in_level = (experience.total - lo).max(0);
    let span = (hi - lo).max(1);
    let progress = experience.progress_in_level();

    if let Ok(mut text) = level_q.single_mut() {
        **text = format!("LV {}", experience.level);
    }
    if let Ok(mut text) = progress_q.single_mut() {
        **text = format!("{} / {}", in_level, span);
    }
    if let Ok(mut node) = fill_q.single_mut() {
        node.width = Val::Percent(progress * 100.0);
    }
}
