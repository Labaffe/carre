use crate::game_manager::state::GameState;
use bevy::{prelude::*, scene::ron::value};

pub struct ScorePlugin;

impl Plugin for ScorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Score>()
            .init_resource::<Level>()
            .add_systems(OnEnter(GameState::Playing), setup_score_ui)
            .add_systems(OnExit(GameState::Playing), cleanup_score_ui)
            .add_systems(
                Update,
                (
                    score_update.run_if(in_state(GameState::Playing)),
                    level_update.run_if(in_state(GameState::Playing)),
                ),
            );
    }
}

#[derive(Component)]
struct ScoreUI;

#[derive(Component)]
struct ScoreText;
#[derive(Component)]
struct LevelText;
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
#[derive(Resource)]
pub struct Level {
    value: usize,
}

const LEVELS: [i32; 4] = [50, 100, 150, 200];

impl Default for Level {
    fn default() -> Self {
        Self { value: 0 }
    }
}
fn setup_score_ui(
    mut commands: Commands,
    mut score: ResMut<Score>,
    mut level: ResMut<Level>,
    asset_server: Res<AssetServer>,
) {
    *score = Score::default();
    *level = Level::default();
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
                Text::new("OVER 9000"),
                TextFont { font: font.clone(), font_size: 90.0, ..default() },
                TextColor(Color::srgba(1.0, 0.0, 0.0, 1.0)),
                BaseFontSize(90.0),
                ScoreText,
            ));
            parent.spawn((
                Text::new("level"),
                TextFont { font: font.clone(), font_size: 90.0, ..default() },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
                BaseFontSize(90.0),
                LevelText,
            ));
        });
}

fn cleanup_score_ui(mut commands: Commands, query: Query<Entity, With<ScoreUI>>) {
    for entity in query.iter() {
        if let Ok(mut e) = commands.get_entity(entity) {
            e.despawn();
        }
    }
}

fn score_update(
    time: Res<Time>,
    mut text_q: Query<&mut Text, With<ScoreText>>,
    mut font_q: Query<(&mut TextFont, &BaseFontSize), With<ScoreText>>,
    mut score: ResMut<Score>,
) {
    score.current_time += time.delta_secs();
    for mut text in text_q.iter_mut() {
        **text = score.text();
    }
    let coef = score.get_size_coeff();
    let scale = 0.3 * coef + 1.0 * (1.0 - coef);
    for (mut font, base) in font_q.iter_mut() {
        font.font_size = base.0 * scale;
    }
}
fn level_update(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    mut text_q: Query<&mut Text, With<LevelText>>,
    mut font_q: Query<(&mut TextFont, &BaseFontSize), With<LevelText>>,
    mut level: ResMut<Level>,
    score: Res<Score>,
) {
    let levelup = score.value > LEVELS[level.value] && LEVELS.len() > level.value + 1;
    if levelup {
        level.value += 1;
        commands.spawn((AudioPlayer::new(asset_server.load("audio/sfx/level_up.ogg")), PlaybackSettings::DESPAWN));
    }
    for mut text in text_q.iter_mut() {
        **text = level.value.to_string();
    }
    let scale = if levelup { 1.0 } else { 0.3 };
    for (mut font, base) in font_q.iter_mut() {
        font.font_size = base.0 * scale;
    }
}
