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
            .init_resource::<HandSince>()
            .add_systems(Update, (
                animate_hand,
                hover_card,
                hide_hand
            ).run_if(in_state(GameState::Playing)));
    }
}

#[derive(Resource, Default)]
pub struct HandVisible(pub bool);

#[derive(Resource, Default)]
pub struct HandSince(pub f32);

use crate::tweening::{TweenSequence, Ease,StyleLeft,Tween,StyleTop};
fn hide_hand(
    time:Res<Time>,
    since:Res<HandSince>,
    mut visible:ResMut<HandVisible>
) {
    if (time.elapsed_seconds()>since.0+5.0) & visible.0 {
        visible.0 = false;
    }
}
fn animate_hand(
    mut commands: Commands,
    visible: Res<HandVisible>,
    windows: Query<&Window>,
    mut query: Query<(Entity,&HandCard, &CardUI, &mut Style)>,
) {
    if !visible.is_changed() {
        return;
    }
    let window = windows.single();
    let count = query.iter().len();
    let mut i = 0;
    println!("{}, {}",visible.0,visible.is_changed());
    println!("{}",query.iter().len());
    for (entity,_, card_ui,mut style) in query.iter_mut() {
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
        let base_y = window.height() * 0.5;

        style.top = Val::Px(base_y);
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