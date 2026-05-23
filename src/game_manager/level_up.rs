//! Level-up via paliers de score → choix de carte.
//!
//! Quand le score franchit un palier (`threshold_for_level(current_level + 1)`),
//! le jeu est mis en pause via `Time<Virtual>::pause()` et un modal s'affiche
//! avec 3 cartes piochées aléatoirement dans le pool de
//! [crate::deckbuilding::cards]. Le clic sur l'une applique son `CardEffect`
//! au joueur, le modal se ferme, le jeu reprend.
//!
//! Paliers croissants : `25 * n * (n+1)` → 50, 150, 300, 500, 750, 1050…
//! Les premiers paliers sont rapides à atteindre puis la fréquence diminue.

use bevy::prelude::*;

use crate::deckbuilding::cards::{card_pool, Card, CardEffect, CardType};
use crate::game_manager::state::GameState;
use crate::player::player::{Armor, Player, PlayerStats};
use crate::player::power::{EquippedPower, PowerKind};
use crate::ui::score::Score;
use crate::weapon::weapon::{Weapon, WeaponKind};

pub struct LevelUpPlugin;

impl Plugin for LevelUpPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LevelUpTracker>()
            .init_resource::<CardSelectState>()
            .add_systems(OnEnter(GameState::Playing), reset_tracker)
            .add_systems(
                Update,
                (
                    watch_score.run_if(in_state(GameState::Playing)),
                    handle_card_click.run_if(in_state(GameState::Playing)),
                    update_card_hover.run_if(in_state(GameState::Playing)),
                ),
            );
    }
}

#[derive(Resource, Default)]
pub struct LevelUpTracker {
    /// Nombre de paliers déjà franchis (= nb de cartes choisies dans la run).
    pub current_level: i32,
}

#[derive(Resource, Default)]
pub struct CardSelectState {
    pub active: bool,
}

/// Palier de score à atteindre pour passer du niveau (n-1) au niveau n.
/// Formule : `25 * n * (n+1)` → 50, 150, 300, 500, 750, 1050, 1400…
fn threshold_for_level(n: i32) -> i32 {
    25 * n * (n + 1)
}

fn reset_tracker(
    mut tracker: ResMut<LevelUpTracker>,
    mut state: ResMut<CardSelectState>,
) {
    *tracker = LevelUpTracker::default();
    *state = CardSelectState::default();
}

fn watch_score(
    score: Res<Score>,
    mut tracker: ResMut<LevelUpTracker>,
    mut state: ResMut<CardSelectState>,
    mut time: ResMut<Time<Virtual>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    player_q: Query<(&Weapon, &EquippedPower), With<Player>>,
) {
    if state.active {
        return;
    }
    let next_threshold = threshold_for_level(tracker.current_level + 1);
    if score.value() >= next_threshold {
        // Récupère l'arme et le pouvoir équipés pour exclure les "swap vers
        // ce que tu as déjà" du pool de propositions.
        let (current_weapon, current_power) = match player_q.single() {
            Ok((w, p)) => (Some(w.0), Some(p.0)),
            Err(_) => (None, None),
        };
        tracker.current_level += 1;
        state.active = true;
        time.pause();
        let cards = sample_three_cards(current_weapon, current_power);
        spawn_card_modal(&mut commands, &asset_server, &cards);
    }
}

/// Pioche jusqu'à 3 cartes distinctes du pool, en excluant les cartes qui
/// proposent l'arme ou le pouvoir DÉJÀ équipés (inutile de re-équiper le même).
/// Les cartes passives (stats) restent toujours disponibles, elles cumulent.
fn sample_three_cards(
    current_weapon: Option<WeaponKind>,
    current_power: Option<PowerKind>,
) -> Vec<Card> {
    let pool: Vec<Card> = card_pool()
        .into_iter()
        .filter(|c| match c.effect {
            CardEffect::SwapWeapon(k) => Some(k) != current_weapon,
            CardEffect::SwapPower(k) => Some(k) != current_power,
            _ => true,
        })
        .collect();
    let n = pool.len();
    let target = 3.min(n);
    let mut indices: Vec<usize> = Vec::with_capacity(target);
    while indices.len() < target {
        let i = fastrand::usize(..n);
        if !indices.contains(&i) {
            indices.push(i);
        }
    }
    indices.into_iter().map(|i| pool[i].clone()).collect()
}

#[derive(Component)]
struct CardModalRoot;

#[derive(Component)]
struct CardChoice {
    effect: CardEffect,
    base_color: Color,
}

fn card_color(card_type: &CardType) -> Color {
    match card_type {
        CardType::Primary => Color::srgba(0.70, 0.22, 0.22, 1.0),
        CardType::Secondary => Color::srgba(0.20, 0.40, 0.75, 1.0),
        CardType::Passive => Color::srgba(0.30, 0.60, 0.30, 1.0),
    }
}

fn spawn_card_modal(commands: &mut Commands, asset_server: &AssetServer, cards: &[Card]) {
    let font = asset_server.load("fonts/PressStart2P-Regular.ttf");

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
                column_gap: Val::Px(28.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.78)),
            GlobalZIndex(1000),
            CardModalRoot,
        ))
        .with_children(|parent| {
            // Titre "LEVEL UP !" au-dessus des cartes — placé en absolute pour
            // ne pas perturber l'alignement horizontal des cartes.
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(60.0),
                    left: Val::Percent(0.0),
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                children![(
                    Text::new("LEVEL UP — choisis une carte"),
                    TextFont { font: font.clone(), font_size: 36.0, ..default() },
                    TextColor(Color::srgba(1.0, 0.9, 0.2, 1.0)),
                )],
            ));

            for card in cards.iter() {
                let color = card_color(&card.card_type);
                parent
                    .spawn((
                        Button,
                        Node {
                            width: Val::Px(240.0),
                            height: Val::Px(340.0),
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::FlexStart,
                            align_items: AlignItems::Center,
                            padding: UiRect::all(Val::Px(20.0)),
                            row_gap: Val::Px(16.0),
                            border: UiRect::all(Val::Px(3.0)),
                            ..default()
                        },
                        BorderColor::all(Color::srgba(0.0, 0.0, 0.0, 0.6)),
                        BackgroundColor(color),
                        CardChoice { effect: card.effect, base_color: color },
                    ))
                    .with_children(|p| {
                        p.spawn((
                            Text::new(&card.name),
                            TextFont { font: font.clone(), font_size: 22.0, ..default() },
                            TextColor(Color::WHITE),
                        ));
                        p.spawn((
                            Text::new(card.card_type.to_string()),
                            TextFont { font: font.clone(), font_size: 12.0, ..default() },
                            TextColor(Color::srgba(1.0, 1.0, 1.0, 0.7)),
                        ));
                        p.spawn((
                            Text::new(&card.description),
                            TextFont { font: font.clone(), font_size: 14.0, ..default() },
                            TextColor(Color::WHITE),
                        ));
                    });
            }
        });
}

/// Met en avant la carte survolée (assombrit légèrement, bordure plus claire).
fn update_card_hover(
    mut q: Query<(&Interaction, &CardChoice, &mut BackgroundColor, &mut BorderColor), Changed<Interaction>>,
) {
    for (interaction, choice, mut bg, mut border) in q.iter_mut() {
        match *interaction {
            Interaction::Hovered => {
                bg.0 = choice.base_color.with_luminance(0.55);
                *border = BorderColor::all(Color::srgba(1.0, 0.9, 0.2, 1.0));
            }
            Interaction::Pressed => {
                bg.0 = choice.base_color.with_luminance(0.35);
            }
            Interaction::None => {
                bg.0 = choice.base_color;
                *border = BorderColor::all(Color::srgba(0.0, 0.0, 0.0, 0.6));
            }
        }
    }
}

fn handle_card_click(
    mut commands: Commands,
    mut state: ResMut<CardSelectState>,
    mut time: ResMut<Time<Virtual>>,
    interactions: Query<(&Interaction, &CardChoice), Changed<Interaction>>,
    modal_q: Query<Entity, With<CardModalRoot>>,
    mut stats_q: Query<&mut PlayerStats, With<Player>>,
    mut weapon_q: Query<&mut Weapon, With<Player>>,
    mut power_q: Query<&mut EquippedPower, With<Player>>,
    mut armor_q: Query<&mut Armor, With<Player>>,
) {
    if !state.active {
        return;
    }
    for (interaction, choice) in interactions.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        apply_effect(
            choice.effect,
            &mut stats_q,
            &mut weapon_q,
            &mut power_q,
            &mut armor_q,
        );
        for e in modal_q.iter() {
            if let Ok(mut ec) = commands.get_entity(e) {
                ec.try_despawn();
            }
        }
        state.active = false;
        time.unpause();
        return;
    }
}

fn apply_effect(
    effect: CardEffect,
    stats_q: &mut Query<&mut PlayerStats, With<Player>>,
    weapon_q: &mut Query<&mut Weapon, With<Player>>,
    power_q: &mut Query<&mut EquippedPower, With<Player>>,
    armor_q: &mut Query<&mut Armor, With<Player>>,
) {
    match effect {
        CardEffect::SpeedBoost { factor } => {
            if let Ok(mut stats) = stats_q.single_mut() {
                stats.speed_mult *= factor;
            }
        }
        CardEffect::RapidFire { factor } => {
            if let Ok(mut stats) = stats_q.single_mut() {
                stats.fire_rate_mult *= factor;
            }
        }
        CardEffect::ReinforcedArmor { add } => {
            if let Ok(mut armor) = armor_q.single_mut() {
                armor.max += add;
                armor.current = (armor.current + add).min(armor.max);
            }
        }
        CardEffect::SwapWeapon(kind) => {
            if let Ok(mut w) = weapon_q.single_mut() {
                w.0 = kind;
            }
        }
        CardEffect::SwapPower(kind) => {
            if let Ok(mut p) = power_q.single_mut() {
                p.0 = kind;
            }
        }
    }
}
