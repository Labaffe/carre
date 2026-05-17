use crate::game_manager::state::GameState;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use crate::deckbuilding::cards::{CardType, Card};
use crate::deckbuilding::card_ui::{spawn_card_ui,CardUI,HandCard,PlayedCard,DeckCard,DiscardCard};
use crate::deckbuilding::layout::*;
use crate::tweening::{TweenSequence, Ease,StyleLeft,Tween,StyleTop};
pub struct CardPlayedPlugin;

impl Plugin for CardPlayedPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
                activate_card,
                position_card
            ).run_if(in_state(GameState::Playing)));
    }
}

fn position_card(
    mut query: Query<(Entity,Ref<PlayedCard>,&mut CardUI, &mut Style)>,
    windows: Query<&Window>,
) {
    let window = windows.single();
    let count = query.iter().len();
    let mut max = 0;
    for (entity, played_card,mut card_ui,mut style) in query.iter_mut() {
        if !played_card.is_changed() {
            if card_ui.index > max-1 {
                max = card_ui.index+1;
                println!("card {}",card_ui.index)
            }
        }
    }
    let mut new_i = max;
    for (entity, played_card,mut card_ui,mut style) in query.iter_mut() {
        if played_card.is_changed() {
            card_ui.index = new_i;
            new_i +=1;
            println!("{}",new_i)
        }
    }
    for (entity, played_card,mut card_ui,mut style) in query.iter_mut() {
        style.left = Val::Px( card_center_x(card_ui.index, window, count));
    }
}
fn activate_card(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    time:Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(Entity,Ref<PlayedCard>,&CardUI,&Style)>,
    windows: Query<&Window>,
) {
    let window = windows.single();
    let base_y = window.height() * 1.0- 100.0;
    let mut count = 0;
    for (_, _,_,_) in query.iter_mut() {
        count +=1;
    }
    //let mut i = 0;
    for (entity,played_card,card_ui,style) in query.iter_mut() {
        let from_y = match style.top {
            Val::Px(f) => {f},
            _ => {0.0}
        };
        if played_card.is_changed() {
            commands.entity(entity).remove::<TweenSequence::<StyleTop>>().insert(
                TweenSequence::<StyleTop>::new(
                    Tween::new(from_y, base_y, 0.5, Ease::OutQuad)
                )
            );
        }
        if mouse.is_changed() {
            if card_ui.card.card_type == CardType::Primary{
                if mouse.pressed(MouseButton::Left) {
                    commands.entity(entity).remove::<TweenSequence::<StyleTop>>().insert(
                        TweenSequence::<StyleTop>::new(
                            Tween::new(base_y, base_y - 30.0, 0.05, Ease::OutQuad)
                        )
                    );
                }   
                else {
                    commands.entity(entity).remove::<TweenSequence::<StyleTop>>().insert(
                        TweenSequence::<StyleTop>::new(
                            Tween::new(base_y - 30.0, base_y, 0.3, Ease::OutQuad)
                        )
                    );
                }
            }
        }
        if keyboard.pressed(KeyCode::Space) {
            if card_ui.card.card_type == CardType::Secondary{
                if mouse.pressed(MouseButton::Left) {
                    commands.entity(entity).remove::<TweenSequence::<StyleTop>>().insert(
                        TweenSequence::<StyleTop>::new(
                            Tween::new(base_y, base_y - 30.0, 0.05, Ease::OutQuad)
                        )
                    );
                }   
                else {
                    commands.entity(entity).remove::<TweenSequence::<StyleTop>>().insert(
                        TweenSequence::<StyleTop>::new(
                            Tween::new(base_y - 30.0, base_y, 0.3, Ease::OutQuad)
                        )
                    );
                }
            }
        }
        
    }
}