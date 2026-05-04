mod editor;
mod system;
use bevy::prelude::*;
use crate::{editor::system::*, game_manager::state::GameState};
pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Editor), (init))
        .add_systems(Update, (pause).run_if(in_state(GameState::Editor)));
    }
}