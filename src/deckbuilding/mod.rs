//! Deckbuilding :
//! Choix de cartes à chaque passage de niveau 

pub mod card_hand;
pub mod card_played;
mod card_deck;
mod cards;
mod card_ui;
mod layout;

pub use crate::deckbuilding::card_hand::{CardHandPlugin};
pub use crate::deckbuilding::card_played::{CardPlayedPlugin};
pub use crate::deckbuilding::card_deck::{CardDeckPlugin};


use crate::game_manager::state::GameState;
use bevy::prelude::*;


pub struct DeckbuildingPlugin;

impl Plugin for DeckbuildingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            CardHandPlugin,
            CardPlayedPlugin,
            CardDeckPlugin
        ));
    }
}