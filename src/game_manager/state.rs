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
    /// Écran de chargement court (~0.6s) joué avant l'entrée en Playing.
    /// Masque les hitches initiaux (asset decode, glyph rasterization,
    /// musique). Routé par levelselect/gameover restart avant Playing.
    Loading,
    Editor,
    /// Écran de fin "Merci d'avoir joué".
    Credits,
    GameOver,
}
