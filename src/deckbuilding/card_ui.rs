use crate::deckbuilding::cards::Card;
use bevy::prelude::*;

#[derive(Component)]
pub struct CardUI(i32);
#[derive(Component)]
struct CardName;
#[derive(Component)]
struct CardRequirement;
#[derive(Component)]
struct CardType;
#[derive(Component)]
struct Description;


pub fn spawn_card_ui<T:Card>(
    mut commands: Commands,
    asset_server:AssetServer, 
    card:T, 
    index:i32
) {
    let font = asset_server.load("fonts/PressStart2P-Regular.ttf");
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    top: Val::Px(20.0),
                    right: Val::Px(20.0),
                    column_gap: Val::Px(12.0),
                    
                    ..default()
                },
                // fond entièrement noir au départ
                background_color: Color::rgba(0.0, 0.0, 0.0, 1.0).into(),
                visibility: Visibility::Hidden,
                ..default()
            },
            CardUI(index),
        ))
        .with_children(|parent| {
            // texte invisible au départ (alpha = 0, scale réduit via Transform)
            parent.spawn((
                TextBundle::from_section(
                    card.name(),
                    TextStyle {
                        font: font.clone(),
                        font_size: 90.0,
                        color: Color::rgba(1.0, 0.0, 0.0, 1.0),
                    },
                ),
                CardName,
            ));
            parent.spawn((
                TextBundle::from_section(
                    card.requirement().to_string(),
                    TextStyle {
                        font: font.clone(),
                        font_size: 90.0,
                        color: Color::rgba(1.0, 1.0, 1.0, 1.0),
                    },
                ),
                CardRequirement,
            ));
        });
}

