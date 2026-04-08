use bevy::prelude::*;

pub struct CrosshairPlugin;

impl Plugin for CrosshairPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_crosshair)
            .add_systems(Update, crosshair_follow_mouse);
    }
}

#[derive(Component)]
pub struct Crosshair;

fn setup_crosshair(mut commands: Commands) {
    let h_bar = commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(1.0, 0.0, 0.0),
                custom_size: Some(Vec2::new(10.0, 2.0)),
                ..default()
            },
            ..default()
        })
        .id();

    let v_bar = commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(1.0, 0.0, 0.0),
                custom_size: Some(Vec2::new(2.0, 10.0)),
                ..default()
            },
            ..default()
        })
        .id();

    commands
        .spawn((SpatialBundle::default(), Crosshair))
        .push_children(&[h_bar, v_bar]);
}

fn crosshair_follow_mouse(
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut crosshair_q: Query<&mut Transform, With<Crosshair>>,
) {
    let window = windows.single();
    let (camera, camera_transform) = camera_q.single();

    if let Some(cursor_pos) = window.cursor_position() {
        if let Some(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
            crosshair_q.single_mut().translation = world_pos.extend(1.0);
        }
    }
}
