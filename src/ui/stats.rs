//! UI des stats du joueur (PlayerStats + Armor.max).
//!
//! Affiche en bas-droite : vitesse, cadence de tir et armure max. Les valeurs
//! qui dévient de la base (1.0× pour les multipliers, `PLAYER_MAX_ARMOR` pour
//! l'armure) sont colorées en vert pour signaler un boost actif.

use bevy::prelude::*;

use crate::game_manager::state::GameState;
use crate::player::player::{Armor, Player, PlayerStats, PLAYER_MAX_ARMOR};

pub struct StatsUiPlugin;

impl Plugin for StatsUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), setup_stats_ui)
            // Cleanup centralisé via `cleanup_playing` (GameplayEntity).
            .add_systems(
                Update,
                update_stats_ui.run_if(in_state(GameState::Playing)),
            );
    }
}

#[derive(Component)]
#[require(crate::GameplayEntity)]
struct StatsUI;

#[derive(Component)]
struct SpeedValueText;
#[derive(Component)]
struct FireValueText;
#[derive(Component)]
struct ArmorMaxValueText;

const LABEL_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 0.75);
const BASE_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 1.0);
const BOOST_COLOR: Color = Color::srgba(0.3, 1.0, 0.4, 1.0);

fn setup_stats_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/PressStart2P-Regular.ttf");

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(20.0),
                right: Val::Px(20.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(12.0)),
                row_gap: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            StatsUI,
        ))
        .with_children(|p| {
            spawn_stat_row(p, &font, "SPD", "x1.00", SpeedValueText);
            spawn_stat_row(p, &font, "FIRE", "x1.00", FireValueText);
            spawn_stat_row(
                p,
                &font,
                "ARM",
                &PLAYER_MAX_ARMOR.to_string(),
                ArmorMaxValueText,
            );
        });
}

/// Spawn une ligne `LABEL  value` dans le conteneur stats. `marker` est le
/// composant identifiant le `Text` à update chaque frame.
fn spawn_stat_row<M: Component>(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    label: &str,
    initial_value: &str,
    marker: M,
) {
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(10.0),
                ..default()
            },
        ))
        .with_children(|row| {
            row.spawn((
                Text::new(label),
                TextFont { font: font.clone(), font_size: 16.0, ..default() },
                TextColor(LABEL_COLOR),
            ));
            row.spawn((
                Text::new(initial_value),
                TextFont { font: font.clone(), font_size: 16.0, ..default() },
                TextColor(BASE_COLOR),
                marker,
            ));
        });
}

fn update_stats_ui(
    player_q: Query<(&PlayerStats, &Armor), With<Player>>,
    mut speed_q: Query<
        (&mut Text, &mut TextColor),
        (With<SpeedValueText>, Without<FireValueText>, Without<ArmorMaxValueText>),
    >,
    mut fire_q: Query<
        (&mut Text, &mut TextColor),
        (With<FireValueText>, Without<SpeedValueText>, Without<ArmorMaxValueText>),
    >,
    mut armor_q: Query<
        (&mut Text, &mut TextColor),
        (With<ArmorMaxValueText>, Without<SpeedValueText>, Without<FireValueText>),
    >,
) {
    let Ok((stats, armor)) = player_q.single() else { return };

    if let Ok((mut text, mut color)) = speed_q.single_mut() {
        **text = format!("x{:.2}", stats.speed_mult);
        color.0 = if stats.speed_mult > 1.0 { BOOST_COLOR } else { BASE_COLOR };
    }
    // Pour FIRE : un multiplier < 1.0 = cadence PLUS RAPIDE (= boost).
    if let Ok((mut text, mut color)) = fire_q.single_mut() {
        **text = format!("x{:.2}", stats.fire_rate_mult);
        color.0 = if stats.fire_rate_mult < 1.0 { BOOST_COLOR } else { BASE_COLOR };
    }
    if let Ok((mut text, mut color)) = armor_q.single_mut() {
        **text = armor.max.to_string();
        color.0 = if armor.max > PLAYER_MAX_ARMOR { BOOST_COLOR } else { BASE_COLOR };
    }
}
