
/// Position de spawn d'un ennemi.
#[derive(Clone, Copy, Debug)]
pub enum SpawnPosition {
    /// Position aléatoire sur le bord haut de l'écran (défaut pour les UFOs).
    Top,
    /// Position aléatoire sur le bord bas.
    Bottom,
    /// Position aléatoire sur le bord gauche.
    Left,
    /// Position aléatoire sur le bord droit.
    Right,
    /// Position exacte en pixels (x, y).
    At(f32, f32),
}

impl SpawnPosition {
    /// Résout la position de spawn en coordonnées monde.
    /// `margin` = marge intérieure par rapport au bord.
    pub fn resolve(self, window: &bevy::window::Window, margin: f32) -> bevy::math::Vec2 {
        let half_w = window.width() / 2.0 - margin;
        let half_h = window.height() / 2.0;
        match self {
            SpawnPosition::Top => {
                let x = (fastrand::f32() - 0.5) * 2.0 * half_w;
                bevy::math::Vec2::new(x, half_h + 40.0)
            }
            SpawnPosition::Bottom => {
                let x = (fastrand::f32() - 0.5) * 2.0 * half_w;
                bevy::math::Vec2::new(x, -half_h - 40.0)
            }
            SpawnPosition::Left => {
                let y = (fastrand::f32() - 0.5) * 2.0 * half_h;
                bevy::math::Vec2::new(-half_w - 40.0, y)
            }
            SpawnPosition::Right => {
                let y = (fastrand::f32() - 0.5) * 2.0 * half_h;
                bevy::math::Vec2::new(half_w + 40.0, y)
            }
            SpawnPosition::At(x, y) => bevy::math::Vec2::new(x, y),
        }
    }
}

impl Default for SpawnPosition {
    fn default() -> Self {
        SpawnPosition::Top
    }
}