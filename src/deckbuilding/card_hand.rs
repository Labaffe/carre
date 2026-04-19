use crate::game_manager::state::GameState;
use bevy::prelude::*;
use crate::deckbuilding::cards::Card;
use crate::deckbuilding::card_ui::{spawn_card_ui,CardUI,HandCard,PlayedCard,DeckCard,DiscardCard};
pub struct CardHandPlugin;
use crate::deckbuilding::layout::card_center_x;

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
        spawn_card_ui(commands.reborrow(), asset_server.clone(), Card::new(), i);
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

use crate::tweening::{TweenSequence, Ease,StyleLeft,Tween,StyleTop};
fn animate_hand(
    mut commands: Commands,
    visible: Res<HandVisible>,
    windows: Query<&Window>,
    query: Query<(Entity,&HandCard, &CardUI)>,
) {
    if !visible.is_changed() {
        return;
    }
    let window = windows.single();
    let count = query.iter().len();
    let mut i = 0;
    for (entity,_, card_ui) in query.iter() {
        let target_x = if visible.0 {
            card_center_x(i,window,count)
        } else {
            -300.0 //- i * 200.0 // exit left
        };
        let from_x = if visible.0 {
            window.width()+300.0// - i * 200.0
        } else {
            card_center_x(i,window,count)
        };
        
        commands.entity(entity).insert(
            TweenSequence::<StyleLeft>::new(
                Tween::new(from_x, from_x,  (i as f32) * 0.1, Ease::OutQuad)
            ).then(
                Tween::new(target_x, target_x-200.0, 5.0, Ease::Linear)
            ).then(
                Tween::new(from_x, target_x, 0.5, Ease::OutQuad)
            )
        );
        i += 1;
    }
}
// In animate_hand or a new system
fn hover_card(
    mut commands: Commands,
    mut interaction_query: Query<(Entity, &Interaction,&HandCard,&mut CardUI), (Changed<Interaction>, With<CardUI>)>,
    windows: Query<&Window>,
) {
    let window = windows.single();
    let base_y = window.height() * 0.5;
    let mut count = 0;
    for (entity, interaction,_,mut card_ui) in interaction_query.iter_mut() {
        count +=1;
    }
    let mut i = 0;
    for (entity, interaction,_,mut card_ui) in interaction_query.iter_mut() {
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
                commands.entity(entity).remove::<HandCard>().insert(PlayedCard {});
            }
            
            Interaction::None => {
                commands.entity(entity).insert(
                    TweenSequence::<StyleTop>::new(
                        Tween::new(base_y - 30.0, base_y, 0.15, Ease::OutQuad)
                    )
                );
                
            }
        }
        i += 1;
    }
}