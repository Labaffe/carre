use crate::deckbuilding::cards::{Card,CardType};
use bevy::prelude::*;

#[derive(Component)]
pub struct CardUI {
    pub index:i32,
    pub selectable:bool,
    pub played:bool,
    pub card:Card
}
#[derive(Component)]
pub struct HandCard {}
#[derive(Component)]
pub struct DeckCard {}
#[derive(Component)]
pub struct PlayedCard {}
#[derive(Component)]
pub struct DiscardCard {}


pub fn spawn_card_ui(
    mut commands: Commands,
    asset_server: AssetServer,
    card: Card,
    index: i32,
) {
    let font = asset_server.load("fonts/PressStart2P-Regular.ttf");
    let color = match card.card_type {
        CardType::Primary => {Color::rgb(0.4, 0.1, 0.1)},
        CardType::Secondary => {Color::rgb(0.1, 0.4, 0.1)},
        CardType::Passive => {Color::rgb(0.1, 0.1, 0.4)}
    };
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Px(200.0),
                    height: Val::Px(300.0),
                    top: Val::Px(3000.0),
                    left: Val::Px(3000.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(10.0)),
                    position_type: PositionType::Absolute,
                    ..default()
                },
                z_index: ZIndex::Global(11),
                background_color: color.into(),
                //transform: Transform::from_translation(Vec3::new(1000.0, 300.0, 0.0)),
                ..default()
            },
            CardUI{index,selectable:false,played:false,card:card.clone()},
            Interaction::default(), 
            DeckCard {}
        ))
        .with_children(|parent| {
            // Title (top)
            parent.spawn(TextBundle::from_section(
                card.name,
                TextStyle {
                    font: font.clone(),
                    font_size: 24.0,
                    color: Color::WHITE,
                },
            ));

            // Spacer / description
            parent.spawn(TextBundle::from_section(
                card.description,
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
                    card.card_type.to_string(),
                    TextStyle {
                        font: font.clone(),
                        font_size: 16.0,
                        color: Color::YELLOW,
                    },
                ));

                row.spawn(TextBundle::from_section(
                    card.requirement.to_string(),
                    TextStyle {
                        font,
                        font_size: 16.0,
                        color: Color::CYAN,
                    },
                ));
            });
        });
    
    }

