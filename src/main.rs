use bevy::prelude::*;

mod asteroid;
mod background;
mod collision;
mod crosshair;
mod debug;
mod difficulty;
mod explosion;
mod gameover;
mod missile;
mod player;
mod state;
mod weapon;

use asteroid::AsteroidPlugin;
use background::BackgroundPlugin;
use collision::CollisionPlugin;
use crosshair::CrosshairPlugin;
use debug::DebugPlugin;
use difficulty::DifficultyPlugin;
use explosion::ExplosionPlugin;
use gameover::GameOverPlugin;
use missile::MissilePlugin;
use player::PlayerPlugin;
use state::GameState;
use weapon::WeaponPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<GameState>()
        .add_plugins((
            BackgroundPlugin,
            CrosshairPlugin,
            PlayerPlugin,
            AsteroidPlugin,
            CollisionPlugin,
            GameOverPlugin,
            DebugPlugin,
            DifficultyPlugin,
            MissilePlugin,
            ExplosionPlugin,
            WeaponPlugin,
        ))
        .add_systems(Startup, setup)
        .run();
}

#[derive(Component)]
pub struct MusicMain;

#[derive(Component)]
pub struct MusicGameOver;

fn setup(mut commands: Commands, mut windows: Query<&mut Window>, asset_server: Res<AssetServer>) {
    windows.single_mut().cursor.visible = false;

    commands.spawn(Camera2dBundle::default());

    spawn_main_music(&mut commands, &asset_server);
}

pub fn spawn_main_music(commands: &mut Commands, asset_server: &Res<AssetServer>) {
    commands.spawn((
        AudioBundle {
            source: asset_server.load("audio/gradius.ogg"),
            settings: PlaybackSettings::LOOP,
        },
        MusicMain,
    ));
}
