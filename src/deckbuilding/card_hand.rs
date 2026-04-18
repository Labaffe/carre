use crate::game_manager::state::GameState;
use bevy::prelude::*;
use crate::deckbuilding::cards::DummyCard;
use crate::deckbuilding::card_ui::{spawn_card_ui,CardUI};
pub struct CardHandPlugin;

impl Plugin for CardHandPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<HandVisible>()
            .add_systems(OnEnter(GameState::Playing), spawn_hand_ui)
            .add_systems(Update, (
                toggle_hand,
                animate_hand,
                hover_card
            ).run_if(in_state(GameState::Playing)));
    }
}


fn spawn_hand_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    for i in 0..5 {
        spawn_card_ui(commands.reborrow(), asset_server.clone(), DummyCard{}, i);
    }
}

#[derive(Resource, Default)]
pub struct HandVisible(pub bool);

fn toggle_hand(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut visible: ResMut<HandVisible>,
) {
    if keyboard.just_pressed(KeyCode::KeyU) {
        visible.0 = !visible.0;
    }
}
fn card_center_x(i:i32,window:&Window)->f32 {
    let center_x = window.width() / 2.0;
    let spacing = 250.0;
    center_x + ((i as f32) - 2.0) * spacing
}
use crate::tweening::{TweenSequence, Ease,StyleLeft,Tween,StyleTop};
use crate::tweening;
fn animate_hand(
    mut commands: Commands,
    visible: Res<HandVisible>,
    windows: Query<&Window>,
    query: Query<(Entity, &CardUI, &Style)>,
) {
    if !visible.is_changed() {
        return;
    }
    let window = windows.single();
    let center_x = window.width() / 2.0;
    let y = window.height() * 0.5;

    let spacing = 250.0;

    for (entity, card_ui, style) in query.iter() {
        let i = card_ui.index as i32;

        let target_x = if visible.0 {
            card_center_x(i,window)
        } else {
            -300.0 //- i * 200.0 // exit left
        };
        let from_x = if visible.0 {
            window.width()+300.0// - i * 200.0
        } else {
            card_center_x(i,window)
        };
        commands.entity(entity).insert(
            TweenSequence::<StyleLeft>::new(
                Tween::new(from_x, from_x,  (i as f32) * 0.1, Ease::OutQuad)
            ).then(
                Tween::new(target_x, target_x-50.0, 3.5, Ease::OutQuad)
            ).then(
                Tween::new(from_x, target_x, 0.5, Ease::OutQuad)
            )
        );
    }
}
// In animate_hand or a new system
fn hover_card(
    mut commands: Commands,
    mut interaction_query: Query<(Entity, &Interaction, &Style, &CardUI), (Changed<Interaction>, With<CardUI>)>,
    windows: Query<&Window>,
) {
    let window = windows.single();
    let base_y = window.height() * 0.5;

    for (entity, interaction, style,card_ui) in interaction_query.iter_mut() {
        let i = card_ui.index as i32;
        match interaction {
            Interaction::Hovered => {
                commands.entity(entity).insert(
                    TweenSequence::<StyleTop>::new(
                        Tween::new(base_y, base_y - 30.0, 0.2, Ease::OutQuad)
                    )
                );
            }
            Interaction::Pressed => {
                commands.entity(entity).remove::<TweenSequence::<StyleTop>>().remove::<TweenSequence::<StyleLeft>>();
                commands.entity(entity).insert((
                    TweenSequence::<StyleTop>::new(
                        Tween::new(base_y, base_y + 300.0, 0.2, Ease::OutQuad)
                    ),
                    TweenSequence::<StyleLeft>::new(
                        Tween::new(card_center_x(i,window), base_y + 300.0, 0.2, Ease::OutQuad)
                    )
                ));
            }
            Interaction::None => {
                commands.entity(entity).insert(
                    TweenSequence::<StyleTop>::new(
                        Tween::new(base_y - 30.0, base_y, 0.15, Ease::OutQuad)
                    )
                );
            }
            Interaction::Pressed => {}
        }
    }
}