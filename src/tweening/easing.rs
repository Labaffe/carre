#[derive(Clone, Copy)]
pub enum Ease {
    Linear,
    EaseOut,
    EaseIn,
}

impl Ease {
    pub fn sample(self, t: f32) -> f32 {
        match self {
            Ease::Linear => t,
            Ease::EaseOut => 1.0 - (1.0 - t).powi(3),
            Ease::EaseIn => t * t * t,
        }
    }
}