use crate::game_manager::state::GameState;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use crate::deckbuilding::cards::{CardType, Card};
use crate::deckbuilding::card_ui::{spawn_card_ui,CardUI,HandCard,PlayedCard,DeckCard,DiscardCard};
use crate::deckbuilding::card_hand::{HandVisible,HandSince};
use crate::deckbuilding::layout::*;
use crate::tweening::{TweenSequence, Ease,StyleLeft,Tween,StyleTop};
pub struct CardDeckPlugin;

impl Plugin for CardDeckPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(GameState::Playing), spawn_cards)
            .add_systems(Update, (
                draw
            ).run_if(in_state(GameState::Playing)));
    }
}
fn spawn_cards(mut commands: Commands, asset_server: Res<AssetServer>) {
    for i in 0..30 {
        spawn_card_ui(commands.reborrow(), asset_server.clone(), Card::new(), i);
    }
}
fn draw(
    mut commands:Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut visible: ResMut<HandVisible>,
    mut start: ResMut<HandSince>,

    mut query: Query<(Entity,&mut DeckCard,&mut CardUI, &mut Style)>
) {
    if keyboard.just_pressed(KeyCode::KeyU) {
        let size = query.iter().len() as i32;
        let i_s =[
            fastrand::i32(0..size),
            fastrand::i32(0..size),
            fastrand::i32(0..size),
            fastrand::i32(0..size),
            fastrand::i32(0..size)
        ];

        for (entity,mut deck_card,mut card_ui,mut style) in query.iter_mut() {
            
            if i_s.contains(&card_ui.index) {
                commands.entity(entity).remove::<DeckCard>().insert(HandCard {});
            }
        }
        visible.0 = true;
        start.0 = time.elapsed_seconds();
        println!("draw cards");

    }
}