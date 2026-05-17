use crate::deckbuilding::cards::Card;
use bevy::prelude::*;

#[derive(Component)]
pub struct CardUI {
    pub index:i32,
    pub selectable:bool,
    pub played:bool
}
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
            (
            Node {
                    width: Val::Px(200.0),
                    height: Val::Px(300.0),
                    top: Val::Px(300.0),
                    left: Val::Px(300.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(10.0)),
                    position_type: PositionType::Absolute,
                    ..default()
                },
            GlobalZIndex(11),
                BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
                //transform: Transform::from_translation(Vec3::new(1000.0, 300.0, 0.0)),
        ),
            CardUI {index,selectable:false,played:false},
            Interaction::default(), 
        ))
        .with_children(|parent| {
            // Title (top)
            parent.spawn((Text::new(card.name()), TextFont { font: font.clone(), font_size: 24.0, ..default() }, TextColor(Color::WHITE)));

            // Spacer / description
            parent.spawn((Text::new(card.description()), TextFont { font: font.clone(), font_size: 16.0, ..default() }, TextColor(Color::srgb(0.5, 0.5, 0.5))));

            // Bottom row (type + cost)
            parent
                .spawn((
            Node {
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        width: Val::Percent(100.0),
                        ..default()
                    },
        ))
                .with_children(|row| {
                    row.spawn((Text::new(card.card_type().to_string()), TextFont { font: font.clone(), font_size: 16.0, ..default() }, TextColor(Color::srgb(1.0, 1.0, 0.0))));

                    row.spawn((
                        Text::new(card.requirement().to_string()),
                        TextFont { font, font_size: 16.0, ..default() },
                        TextColor(Color::srgb(0.0, 1.0, 1.0)),
                    ));
                });
        });
}
