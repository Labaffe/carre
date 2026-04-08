use crate::difficulty::Difficulty;
use bevy::prelude::*;

pub struct BackgroundPlugin;

impl Plugin for BackgroundPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_background)
            .add_systems(Update, scroll_background);
    }
}

#[derive(Component)]
struct Background;

fn setup_background(mut commands: Commands, asset_server: Res<AssetServer>) {
    let bg = asset_server.load("images/space_background.png");

    for i in 0..2 {
        commands.spawn((
            SpriteBundle {
                texture: bg.clone(),
                transform: Transform::from_xyz(0.0, 1536.0 * i as f32, -1.0),
                ..default()
            },
            Background,
        ));
    }
}

fn scroll_background(
    mut query: Query<&mut Transform, With<Background>>,
    time: Res<Time>,
    difficulty: Res<Difficulty>,
) {
    let base_speed = 150.0;
    let image_height = 1536.0;
    let speed = base_speed * difficulty.factor.powi(2);

    for mut transform in query.iter_mut() {
        transform.translation.y -= speed * time.delta_seconds();

        if transform.translation.y <= -image_height {
            transform.translation.y += image_height * 2.0;
        }
    }
}
