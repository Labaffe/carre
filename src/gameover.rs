use bevy::prelude::*;
use crate::asteroid::Asteroid;
use crate::player::spawn_player;
use crate::state::GameState;

pub struct GameOverPlugin;

impl Plugin for GameOverPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::GameOver), setup_gameover_ui)
            .add_systems(OnExit(GameState::GameOver), cleanup_gameover_ui)
            .add_systems(
                Update,
                handle_restart.run_if(in_state(GameState::GameOver)),
            );
    }
}

#[derive(Component)]
struct GameOverUI;

fn setup_gameover_ui(mut commands: Commands) {
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(20.0),
                    ..default()
                },
                background_color: Color::rgba(0.0, 0.0, 0.0, 0.75).into(),
                ..default()
            },
            GameOverUI,
        ))
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "GAME OVER",
                TextStyle {
                    font_size: 90.0,
                    color: Color::RED,
                    ..default()
                },
            ));
            parent.spawn(TextBundle::from_section(
                "Appuyez sur R pour rejouer",
                TextStyle {
                    font_size: 32.0,
                    color: Color::WHITE,
                    ..default()
                },
            ));
        });
}

fn cleanup_gameover_ui(mut commands: Commands, query: Query<Entity, With<GameOverUI>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn handle_restart(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut commands: Commands,
    asteroids: Query<Entity, With<Asteroid>>,
    asset_server: Res<AssetServer>,
) {
    if keyboard.just_pressed(KeyCode::KeyR) {
        // supprimer tous les astéroïdes restants
        for entity in asteroids.iter() {
            commands.entity(entity).despawn();
        }

        // respawn le joueur
        spawn_player(&mut commands, &asset_server);

        next_state.set(GameState::Playing);
    }
}
