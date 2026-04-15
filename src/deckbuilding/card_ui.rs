use crate::deckbuilding::cards::Card;
use bevy::prelude::*;

#[derive(Component)]
pub struct CardUI(pub i32);
#[derive(Component)]
struct CardName;
#[derive(Component)]
struct CardRequirement;
#[derive(Component)]
struct CardType;
#[derive(Component)]
struct Description;


pub fn spawn_card_ui<T: Card>(
    mut commands: Commands,
    asset_server: AssetServer,
    card: T,
    index: i32,
) {
    let font = asset_server.load("fonts/PressStart2P-Regular.ttf");
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Px(200.0),
                    height: Val::Px(300.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(10.0)),
                    position_type: PositionType::Absolute,
                    ..default()
                },
                background_color: Color::rgb(0.1, 0.1, 0.1).into(),
                ..default()
            },
            CardUI(index),
        ))
        .with_children(|parent| {
            // Title (top)
            parent.spawn(TextBundle::from_section(
                card.name(),
                TextStyle {
                    font: font.clone(),
                    font_size: 24.0,
                    color: Color::WHITE,
                },
            ));

            // Spacer / description
            parent.spawn(TextBundle::from_section(
                card.description(),
                TextStyle {
                    font: font.clone(),
                    font_size: 16.0,
                    color: Color::GRAY,
                },
            ));

            // Bottom row (type + cost)
            parent.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    width: Val::Percent(100.0),
                    ..default()
                },
                ..default()
            })
            .with_children(|row| {
                row.spawn(TextBundle::from_section(
                    card.card_type().to_string(),
                    TextStyle {
                        font: font.clone(),
                        font_size: 16.0,
                        color: Color::YELLOW,
                    },
                ));

                row.spawn(TextBundle::from_section(
                    card.requirement().to_string(),
                    TextStyle {
                        font,
                        font_size: 16.0,
                        color: Color::CYAN,
                    },
                ));
            });
        });
}

