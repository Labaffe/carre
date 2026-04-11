use bevy::prelude::*;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    #[default]
    MainMenu,
    Playing,
    /// État transitoire entre deux niveaux.
    /// Déclenche OnExit(Playing) → cleanup, puis OnEnter(Playing) → setup.
    LevelTransition,
    GameOver,
}
