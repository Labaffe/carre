use bevy::prelude::*;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    #[default]
    MainMenu,
    Playing,
    /// Écran de sélection de niveaux (Campagne ou Primes).
    LevelSelect,
    /// État transitoire entre deux niveaux.
    /// Déclenche OnExit(Playing) → cleanup, puis OnEnter(LevelSelect) → sélecteur.
    LevelTransition,
    /// Écran de fin "Merci d'avoir joué".
    Credits,
    GameOver,
}
