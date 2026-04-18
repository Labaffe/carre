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
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    top: Val::Px(20.0),
                    right: Val::Px(20.0),
                    column_gap: Val::Px(12.0),
                    ..default()
                },
                // fond entièrement noir au départ
                background_color: Color::rgba(0.0, 0.0, 0.0, 0.5).into(),
                ..default()
            },
            ScoreUI,
        ))
        .with_children(|parent| {
            // texte invisible au départ (alpha = 0, scale réduit via Transform)
            parent.spawn((
                TextBundle::from_section(
                    "OVER 9000",
                    TextStyle {
                        font: font.clone(),
                        font_size: 90.0,
                        color: Color::rgba(1.0, 0.0, 0.0, 1.0),
                    },
                ),
                ScoreText,
            ));
            parent.spawn((
                TextBundle::from_section(
                    "level",
                    TextStyle {
                        font: font.clone(),
                        font_size: 90.0,
                        color: Color::rgba(1.0, 1.0, 1.0, 1.0),
                    },
                ),
                LevelText,
            ));
        });
}

fn cleanup_score_ui(mut commands: Commands, query: Query<Entity, With<ScoreUI>>) {
    for entity in query.iter() {
        if let Some(e) = commands.get_entity(entity) {
            e.despawn_recursive();
        }
    }
}

fn score_update(
    time: Res<Time>,
    mut text_q: Query<(&mut Text, &mut Transform), With<ScoreText>>,
    mut score: ResMut<Score>,
) {
    score.current_time += time.delta_seconds();

    // texte : opacité 0 → 1, zoom 0.3 → 1.0
    for (mut text, mut transform) in text_q.iter_mut() {
        for section in text.sections.iter_mut() {
            section.value = score.text();
        }
        let coef = score.get_size_coeff();
        let scale = 0.3 * coef + 1.0 * (1.0 - coef);
        transform.scale = Vec3::splat(scale);
    }
}
fn level_update(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    mut text_q: Query<(&mut Text, &mut Transform), With<LevelText>>,
    mut level: ResMut<Level>,
    score: Res<Score>,
) {
    let levelup = score.value > LEVELS[level.value] && LEVELS.len() > level.value + 1;
    if levelup {
        level.value += 1;
        commands.spawn(AudioBundle {
            source: asset_server.load("audio/sfx/level_up.ogg"),
            settings: PlaybackSettings::DESPAWN,
        });
    }
    // texte : opacité 0 → 1, zoom 0.3 → 1.0
    for (mut text, mut transform) in text_q.iter_mut() {
        for section in text.sections.iter_mut() {
            section.value = (level.value + 1).to_string();
        }
        if levelup {
            transform.scale = Vec3::splat(1.0);
        } else {
            transform.scale = Vec3::splat(0.3);
        }
    }
}
