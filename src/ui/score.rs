use crate::audio::{Sfx, SfxPlayer};
use crate::game_manager::state::GameState;
use bevy::prelude::*;

pub struct ScorePlugin;

impl Plugin for ScorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Score>()
            .init_resource::<Combo>()
            .add_systems(OnEnter(GameState::Playing), setup_score_ui)
            // Cleanup UI : géré centralement par `cleanup_playing` (main.rs)
            // via `#[require(GameplayEntity)]` sur `ScoreUI`.
            .add_systems(
                Update,
                (
                    combo_tick.run_if(in_state(GameState::Playing)),
                    score_update.run_if(in_state(GameState::Playing)),
                    combo_update_ui.run_if(in_state(GameState::Playing)),
                )
                    .chain(),
            );
    }
}

#[derive(Component)]
#[require(crate::GameplayEntity)]
struct ScoreUI;

#[derive(Component)]
struct ScoreText;
#[derive(Component)]
struct ComboText;
/// Taille de police de référence pour le zoom dynamique.
#[derive(Component)]
struct BaseFontSize(f32);

#[derive(Resource)]
pub struct Score {
    value: i32,
    multiplier: i32,
    current_time: f32,
    last_add_time: f32,
}

impl Score {
    pub fn add(self: &mut Self, value_to_add: i32) {
        self.value += value_to_add * self.multiplier;
        self.last_add_time = self.current_time;
    }
    fn get_size_coeff(self: &Self) -> f32 {
        (self.current_time - self.last_add_time).clamp(0.0, 1.0)
    }
    pub fn value(&self) -> i32 {
        self.value
    }
    fn text(self: &Self) -> String {
        self.value.to_string()
    }
}

impl Default for Score {
    fn default() -> Self {
        Score {
            value: 0,
            multiplier: 1,
            current_time: 0.0,
            last_add_time: 0.0,
        }
    }
}

/// Combo de kills consécutifs. Un kill incrémente `count` et reset le timer.
/// Si aucun kill pendant `COMBO_RESET_SECS`, le combo retombe à 0. Un hit
/// reçu par le joueur (même absorbé par l'armor) reset immédiatement.
#[derive(Resource, Default)]
pub struct Combo {
    count: i32,
    time_since_last_kill: f32,
    max_this_run: i32,
    /// Vrai pendant 1 frame quand le multiplicateur vient de passer un palier.
    /// Sert au juice UI (scale + sfx).
    just_leveled_up: bool,
}

const COMBO_RESET_SECS: f32 = 3.0;
/// Paliers : nombre de kills consécutifs requis pour atteindre le multiplicateur.
/// Index = palier (0 → x1, 1 → x2, ...). Doit être strictement croissant.
const COMBO_TIERS: [i32; 5] = [0, 10, 25, 50, 100];

impl Combo {
    /// Appelée à chaque kill d'ennemi/asteroid. Met à jour `Score.multiplier`
    /// pour que les calls suivants à `score.add(...)` bénéficient du combo.
    pub fn on_kill(&mut self, score: &mut Score) {
        let old_mult = self.multiplier();
        self.count += 1;
        self.time_since_last_kill = 0.0;
        if self.count > self.max_this_run {
            self.max_this_run = self.count;
        }
        let new_mult = self.multiplier();
        score.multiplier = new_mult;
        if new_mult > old_mult {
            self.just_leveled_up = true;
        }
    }

    /// Reset à 0 (hit joueur, ou timeout).
    pub fn reset(&mut self, score: &mut Score) {
        self.count = 0;
        self.time_since_last_kill = 0.0;
        score.multiplier = 1;
    }

    pub fn multiplier(&self) -> i32 {
        let mut tier = 1;
        for (i, threshold) in COMBO_TIERS.iter().enumerate() {
            if self.count >= *threshold {
                tier = (i as i32) + 1;
            }
        }
        tier
    }
}

/// Couleur du score selon le multiplicateur actuel.
fn color_for_multiplier(mult: i32) -> Color {
    match mult {
        1 => Color::srgba(1.0, 0.0, 0.0, 1.0),   // rouge (défaut)
        2 => Color::srgba(1.0, 0.55, 0.0, 1.0),  // orange
        3 => Color::srgba(1.0, 0.9, 0.0, 1.0),   // jaune
        4 => Color::srgba(0.3, 1.0, 0.4, 1.0),   // vert vif
        _ => Color::srgba(1.0, 0.84, 0.0, 1.0),  // or
    }
}

fn setup_score_ui(
    mut commands: Commands,
    mut score: ResMut<Score>,
    mut combo: ResMut<Combo>,
    asset_server: Res<AssetServer>,
) {
    *score = Score::default();
    *combo = Combo::default();
    let font = asset_server.load("fonts/PressStart2P-Regular.ttf");
    commands
        .spawn((
            (
            Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(20.0),
                    right: Val::Px(20.0),
                    column_gap: Val::Px(12.0),
                    ..default()
                },
            // fond entièrement noir au départ
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
        ),
            ScoreUI,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(""),
                TextFont { font: font.clone(), font_size: 90.0, ..default() },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
                BaseFontSize(90.0),
                ComboText,
            ));
            parent.spawn((
                Text::new("0"),
                TextFont { font: font.clone(), font_size: 90.0, ..default() },
                TextColor(color_for_multiplier(1)),
                BaseFontSize(90.0),
                ScoreText,
            ));
        });
}

fn combo_tick(time: Res<Time>, mut combo: ResMut<Combo>, mut score: ResMut<Score>) {
    // Reset du flag de palier (consommé par l'UI à la frame précédente).
    combo.just_leveled_up = false;
    if combo.count == 0 {
        return;
    }
    combo.time_since_last_kill += time.delta_secs();
    if combo.time_since_last_kill >= COMBO_RESET_SECS {
        combo.reset(&mut score);
    }
}

fn score_update(
    time: Res<Time>,
    mut text_q: Query<(&mut Text, &mut TextColor), With<ScoreText>>,
    mut font_q: Query<(&mut TextFont, &BaseFontSize), With<ScoreText>>,
    mut score: ResMut<Score>,
) {
    score.current_time += time.delta_secs();
    let target_color = color_for_multiplier(score.multiplier);
    for (mut text, mut color) in text_q.iter_mut() {
        **text = score.text();
        color.0 = target_color;
    }
    let coef = score.get_size_coeff();
    let scale = 0.3 * coef + 1.0 * (1.0 - coef);
    for (mut font, base) in font_q.iter_mut() {
        font.font_size = base.0 * scale;
    }
}

fn combo_update_ui(
    mut text_q: Query<(&mut Text, &mut TextColor), With<ComboText>>,
    mut font_q: Query<(&mut TextFont, &BaseFontSize), With<ComboText>>,
    combo: Res<Combo>,
    mut sfx: SfxPlayer,
) {
    let mult = combo.multiplier();
    let display = if combo.count == 0 {
        String::new()
    } else {
        format!("x{}", mult)
    };
    let color = color_for_multiplier(mult);
    for (mut text, mut text_color) in text_q.iter_mut() {
        **text = display.clone();
        text_color.0 = color;
    }
    let scale = if combo.just_leveled_up { 1.3 } else { 1.0 };
    for (mut font, base) in font_q.iter_mut() {
        font.font_size = base.0 * 0.6 * scale;
    }
    if combo.just_leveled_up {
        sfx.play(Sfx::ScoreMilestone);
    }
}
